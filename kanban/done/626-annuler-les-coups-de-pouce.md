# Annuler les coups de pouce

**Épic :** E20 — Coups de pouce : le panier et l'annulation · **Ordre :** 1 · **Dépend de :** —
**Conception :** `docs/specs/saisie-mobile/E20-cadrage.md` (R3, R4, R5, § 3)

## Objectif

Une erreur de coups de pouce se corrige en **annulant ceux des deux équipes**
et en recommençant par le favori. Après la carte, le domaine sait annuler, la
route existe ; le bouton arrive avec le récapitulatif (628).

L'annulation n'est permise qu'en `PreMatch` et tant qu'**aucune action** n'est
enregistrée : racheter après des actions laisserait celles qui désignent un
joueur temporaire pointer dans le vide (`init_temp_players_use_case.rs:145`,
`:166`), et en `ReadyToPublish` la dépense de trésorerie est figée
(`match_report_ready_to_publish.rs:128-133`).

## Conception

**Domaine** — `MatchReportPreMatch` :

```rust
pub fn cancel_inducements(&self, cancelled_by: CoachId)
    -> Result<(Self, Vec<MatchReportDomainEvent>), DomainError>;
```

- refuse `DomainError::MatchAlreadyStarted` si `home_actions` ou `away_actions`
  n'est pas vide ;
- refuse `DomainError::NoInducementsToCancel` si aucune des deux équipes n'a
  validé (`inducements_for(..).is_none()` des deux côtés) ;
- rend `InducementsCancelled { cancelled_by }`, puis un `TempPlayersReset` par
  camp qui a des joueurs temporaires, construit par `reset_temp_players` — il
  nomme les journaliers retirés ;
- l'état rendu : achats à `None` des deux côtés, `star_engagements` et joueurs
  temporaires vides.

**Rejeu** — `rehydrate`, arête `PreMatch` × `InducementsCancelled` uniquement ;
`ReadyToPublish` × `InducementsCancelled` reste une séquence invalide.

**Use case** — `cancel_inducements_use_case.rs` : charge un **`PreMatch`**
(tout autre état : `NotInPreMatchPhase`, sans `into_pre_match()`), appelle le
domaine, `append_many`, puis `emettre()` chaque événement. `ConcurrentWrite`
devient `Conflict` (carte 623).

**Projection** — un bras dans `match_report_repository.rs` : `home_inducements`
et `away_inducements` à `NULL`.

**Route** — `POST /app/{space_id}/match-report/{match_report_id}/cancel-inducements`,
dans `routes_de_saisie()`. Réponses : `HX-Redirect` vers
`/inducements/{favori}` ; 409 et un message pour `MatchAlreadyStarted`,
`NoInducementsToCancel`, `NotInPreMatchPhase` ; `conflict_response()` pour le
conflit.

## Données de production

Un événement ajouté, aucun modifié, aucune migration. Les flux existants se
rejouent tels quels. Aucun mouvement d'argent : la trésorerie n'est débitée qu'à
la publication. Les journaliers retirés sortent de `players` par
`JourneymenWithdrawn`, le chemin de la carte 455.

## Checklist

- [x] `InducementsCancelled`, son `type_name`, son rejeu
- [x] `cancel_inducements` et ses deux refus
- [x] Le use case, instrumenté, qui émet les trois événements
- [x] Le bras de projection
- [x] La route, dans le groupe gardé
- [x] Tests unitaires : annulation des deux camps, petite monnaie revenue à
      l'écart de valeur, refus avec une action, permis après sa suppression,
      refus sans achat, rejeu, séquence invalide en `ReadyToPublish`, journaliers
      nommés, conflit
- [x] `make lint`, `make check-arch`, `make test`
