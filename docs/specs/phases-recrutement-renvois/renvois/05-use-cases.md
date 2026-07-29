# Renvois — Phase 5 : use cases

**Entrée** : `04-dtos.md` validé.

L'hydratation, `append_batch`, la garde anti-double-application et la traduction des
erreurs sont décrites dans `recrutement/05-use-cases.md`. Ce document consigne les
écarts.

## 1. Use cases de marquage

```rust
// use_cases/mark_player_for_dismissal_use_case.rs
pub enum MarkPlayerForDismissalError {
    TeamNotFound,
    WrongPhase,
    ConcurrentWrite,
    Domain(DomainError),      // dont EligibleFloorReached
    Repository(RepositoryError),
}

pub async fn execute(
    cmd:        MarkPlayerForDismissalCommand,
    team_repo:  &dyn ITeamRepository,
    draft_repo: &dyn IPhaseDraftRepository,
    catalog:    &dyn IRosterCatalogPort,
    squad:      &dyn ISquadPort,
) -> Result<DismissalsDraft, MarkPlayerForDismissalError>
```

Orchestration identique au recrutement : charger `Team`, vérifier la phase
`Dismissals`, hydrater, `draft.mark_player(player_id)?`, persister avec la version.

**Le démarquage réutilise `remove_draft_line_use_case`** du recrutement : retirer une
ligne d'un brouillon par son identifiant est la même opération.

## 2. Le use case de validation

Même forme qu'au recrutement, avec trois différences.

**Pas de vérification de trésorerie** — rien n'entre, rien ne sort. `validate_all()`
ne contrôle que le plancher des 11 éligibles et la possession du staff marqué.

**Le lot est composé de** :
- un `PlayerDismissed` **par joueur** — le renommage de `PlayerFired`, cf. `04-dtos.md`
- un `StaffDismissed` par ligne de staff, **sans `refund_kpo`**
- `DismissalsPhaseValidated` en dernier

**Aucun mouvement de trésorerie n'est produit.** `treasury_movement()` retourne `None`
pour `PlayerDismissed` et pour `StaffDismissed` — c'est précisément ce que le `match`
exhaustif oblige à déclarer, et c'est la traduction en code de « un renvoi ne
rembourse rien ».

La garde anti-double-application fonctionne à l'identique : `DismissalsPhaseValidated`
fait passer l'équipe en `ReadyToPlay`, et `validate_dismissals_phase()` exige la phase
`Dismissals`.

### Une conséquence heureuse

Ce dernier événement déclenche **aussi** la purge des brouillons (D6) et le recalcul
de la valeur d'équipe (carte 251), tous deux abonnés aux entrées en `ReadyToPlay`. La
valeur d'équipe est donc recalculée **après** que les renvoyés ont quitté l'effectif,
sans qu'aucun ordonnancement explicite ne soit nécessaire.

## 3. L'app event de sortie d'effectif

```
Lot appliqué (teams)
   └─► domain event PlayerDismissed        (event store teams)
         └─► publisher teams (couche IO)
               └─► app event PlayerDismissed
                     └─► listener players
                           └─► domain event PlayerDismissed
                                 └─► players_proj.membership = 'Dismissed'
```

### Une course à surveiller

Le recalcul de valeur d'équipe (carte 251) est déclenché par
`DismissalsPhaseValidated` sur le **bus interne de `teams`**, tandis que la sortie
d'effectif transite par l'**app event bus** vers `players`. Rien ne garantit que
`players` ait traité le renvoi quand `teams` recalcule.

**Conséquence** : la valeur d'équipe peut être recalculée en comptant encore les
joueurs renvoyés.

Ce n'est pas bloquant — la valeur converge à l'entrée suivante en `ReadyToPlay`, et
`TeamValueRecomputed` porte une valeur absolue, la dernière gagnant. Mais il faut le
savoir, et c'est un cas de test e2e à écrire explicitement.

Deux façons de le supprimer si on le juge inacceptable : faire dépendre le recalcul de
l'accusé de traitement de `players` (couplage fort, à éviter), ou déclencher un second
recalcul à la réception de `PlayerDismissed` côté `teams` (idempotent, peu coûteux).
**Je penche pour le second**, mais c'est à trancher.

## 4. Le numéro de maillot libéré

Un joueur renvoyé libère son numéro. Comme `players` possède les maillots et attribue
« le premier disponible » au recrutement, la libération est automatique : le joueur
`Dismissed` n'est plus lu par `find_by_team_id`, donc son numéro cesse d'être occupé.

L'ordre des phases fait que ce numéro ne sera réutilisable qu'à la **séquence
suivante** — recrutement puis renvois — ce qui est cohérent avec la règle « on ne
libère pas une place pour recruter dans la même séquence ».

## 5. Règles métier identifiées à cette étape

- **`treasury_movement()` retourne `None` pour les deux événements de renvoi.** C'est
  la traduction en code, vérifiée par le compilateur, de l'absence de remboursement.
- **Le recalcul de valeur d'équipe suit naturellement la validation** : il est abonné
  aux entrées en `ReadyToPlay`, dont `DismissalsPhaseValidated` fait partie.
- **Un joueur marqué compte encore dans les éligibles** jusqu'à l'application du lot.
  C'est ce qui rend l'annulation gratuite, et ce qui fait que le plancher se recalcule
  à chaque marquage.

## 6. Points ouverts pour la phase 6

- La course entre le recalcul de TV et la sortie d'effectif : second recalcul
  idempotent, ou on l'assume ?
- Un rapport de match dépublié pour correction peut référencer un joueur devenu
  `Dismissed`. La série 227-236 ne connaît pas cette notion — à confronter au domaine
  de `match_report` en phase 6.
