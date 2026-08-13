# Phase 7 — Effets de bord — player-detail

## Persistance

### Le panier

Table `players__customisation_baskets`, calquée sur `teams__phase_baskets` :

```sql
player_id  TEXT PRIMARY KEY,
space_id   TEXT        NOT NULL,
state      JSONB       NOT NULL,   -- les lignes, rien d'autre
version    INT         NOT NULL DEFAULT 1,
created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
```

Clé `player_id` seul — le panier est propre au joueur, pas à son auteur
(phase 2). `ICustomisationBasketRepository` : `load` / `save(expected_version)`
/ `delete`, garde de version optimiste.

### Péremption — 24 h d'inactivité, vérifiées à l'ouverture

Un panier abandonné vit **24 h après sa dernière modification**. La fenêtre
glisse sur `updated_at`, pas sur `created_at` : un panier commencé il y a 23 h
mais travaillé à l'instant ne doit pas mourir en pleine session. C'est
l'inactivité qui périme, pas l'ancienneté.

La vérification se fait **à l'ouverture de la fiche**, au seul endroit où
l'information sert : le panier y est de toute façon chargé pour décider du
mode, la péremption ne coûte donc qu'une comparaison sur une donnée déjà en
main. Un cron irait relire des lignes que personne ne regarde.

Conséquences assumées :

**Un `GET` qui écrit.** L'expiration paresseuse fait muter la base sur une
lecture. Sans danger ici — la suppression est idempotente (phase 5), deux
onglets concurrents ne posent pas de problème — mais c'est nommé plutôt que
découvert dans le handler.

**Un message discret au retour**, et non une disparition muette : « votre
saisie de plus de 24 h a été abandonnée ». Le projet répète qu'une chose
escamotée doit se voir ; effacer le travail d'un commissaire sans rien dire
irait contre. Le message est porté par le journal des évolutions au premier
affichage suivant l'expiration.

**Les 24 h sont une règle métier**, donc une constante nommée du domaine du
panier — jamais un `WHERE updated_at < now() - interval '24 hours'` enfoui dans
une requête, où personne ne la trouverait le jour où il faudrait la changer.

**Nettoyage partiel assumé** : un panier abandonné sur un joueur que plus
personne n'ouvre n'est jamais supprimé. Le volume est négligeable — seuls les
paniers abandonnés survivent, la validation et l'annulation supprimant les
autres — et une tâche planifiée coûterait plus que ce qu'elle rendrait.

### Les deltas de caractéristique

Cinq colonnes sur `players_proj` :

```sql
ALTER TABLE players_proj
    ADD COLUMN ma_delta SMALLINT NOT NULL DEFAULT 0,
    ADD COLUMN st_delta SMALLINT NOT NULL DEFAULT 0,
    ADD COLUMN ag_delta SMALLINT NOT NULL DEFAULT 0,
    ADD COLUMN pa_delta SMALLINT NOT NULL DEFAULT 0,
    ADD COLUMN av_delta SMALLINT NOT NULL DEFAULT 0;
```

Signées : une séquelle ou une dégradation les rend négatives.

### Comment le cumul est écrit — le point de la phase 3

La décision était « recalcul depuis l'agrégat, hydratation dans le chemin
d'écriture ». Elle se concrétise plus simplement que prévu, et c'est le choix
des **deltas** qui le permet.

`upsert_player_projection(tx, event)` ne reçoit que la transaction et
l'événement. Mais il peut, **dans la même transaction**, relire les événements
du joueur et reconstruire l'agrégat :

```
append de l'événement
  → relire players_events du joueur (dans la tx)
  → Player::from_events(&events)
  → cumul par caractéristique depuis stat_increases + stat_adjustments
    + offsets de customisation
  → UPDATE players_proj SET *_delta = …
```

**Aucun port n'est injecté dans la transaction**, ce qui était la crainte de la
phase 3. La raison est que le cumul est un **delta** : il ne dépend que des
événements du joueur, jamais du catalogue de postes. Une projection de valeurs
absolues aurait exigé la base du poste, donc `references`, donc un port dans le
chemin d'écriture.

`MatchImpactReverted` est traité gratuitement : `Player::from_events` sait déjà
le défaire, puisque c'est sa raison d'être. Le rejeu, l'ordre et les corrections
de match sont couverts sans code dédié.

Coût assumé : un `SELECT` et un fold par append. Sur un joueur, l'event store
compte quelques dizaines de lignes.

### Lecture

`projection_repository::find_by_team_id` et `find_by_id` exposent les cinq
deltas. `resolve_stats` reste la voie de la fiche joueur — base du poste +
cumul —, mais les consommateurs qui n'ont besoin que du delta (ou d'une valeur
approchée) peuvent désormais le lire en SQL.

---

## Événements

**Quatre événements domaine**, un par customisation, sur le bus interne du BC.

**Aucun app event.** La phase 1 l'a établi : le profil du joueur n'est partagé
par événement nulle part — `PlayersAppEvent` ne porte que
`InitialRosterCompleted` et `PlayerDismissed`. Les consommateurs
(`teams::ISquadPort`, adapters de `match_report`) lisent `players_proj`, donc
la mise à jour de la projection suffit.

**Aucun listener nouveau.** Rien ne réagit à une customisation : ni le calcul
de valeur d'équipe, qui lit la projection au moment où il en a besoin, ni le
journal, qui relit l'event store.

C'est un point à ne pas « corriger » plus tard par réflexe : l'absence d'app
event est une conséquence de l'architecture de lecture, pas un oubli.

---

## Handlers

```rust
// widgets/player_customisation_widget.rs
pub async fn player_customisation_widget(
    Path((space_id, player_id)): Path<(String, String)>,
    auth_session: AuthSession,
    State(state): State<AppState>,
) -> impl IntoResponse
```

Crée le panier s'il n'existe pas — c'est l'entrée dans le mode (phase 5).

```rust
// customisation_controller.rs — les sept POST, même forme
pub async fn post_add_skill(
    Path((space_id, player_id)): Path<(String, String)>,
    auth_session: AuthSession,
    State(state): State<AppState>,
    Form(form): Form<AddSkillForm>,
) -> impl IntoResponse
```

**Chaque handler vérifie l'autorisation.** Masquer le bouton n'est pas un
contrôle d'accès (phase 3).

Traduction des résultats :

| Résultat | Réponse |
|---|---|
| Mutation acceptée | **200** + panneau re-rendu |
| `Domain(_)` — refus métier | **200** + panneau re-rendu portant `RefusalVm` |
| `ConcurrentWrite` | **200** + panneau re-rendu (le panier relu est à jour, le geste est simplement perdu) |
| Validation réussie | **200** + `HX-Trigger: rosterCustomisationSaved` et `HX-Refresh: true` |
| Annulation | **200** + fragment du journal des évolutions |
| Sans droit | **403** |
| Joueur inconnu | **404** |
| Formulaire malformé | **400** |

Le refus métier répond **200**, comme l'endpoint d'édition d'effectif de la
carte 294 et pour la même raison : un 4xx ferait échouer le swap HTMX et
laisserait le commissaire devant un panneau figé.

`ConcurrentWrite` mérite d'être noté : il n'est **pas** présenté comme une
erreur. Le panneau re-rendu porte l'état réel ; le commissaire voit que son
geste n'a pas pris et le refait. Un message d'échec sur un événement aussi rare
qu'invisible ferait plus de bruit que de bien.

---

## Templates

| Template | VM | Notes |
|---|---|---|
| `widgets/player-customisation-widget.html` | `CustomisationVm` | Repris de la maquette pour le **markup et le CSS**. Le JS ne l'est pas — il simulait un panier client (phase 2) |
| `player-detail.html` | `PlayerDetailVm` | Le bouton « ✎ Customiser » perd son `disabled` et gagne son `hx-get` |
| `evolution-journal-widget.html` | inchangé | Cible du retour après annulation |

Le JS restant dans le panneau se réduit à la bascule d'onglets et au filtre de
recherche — le reste est rendu par le serveur.

---

## Tests E2E prévus

Fixture : un joueur d'une équipe de l'espace E2E, et **la seconde identité de
`bypass_auth`** (`X-Bypass-Auth-Profile: simple`) pour les scénarios
d'autorisation — introduite en carte 295, c'est exactement son cas d'usage.

1. **Le bouton n'existe pas pour un membre simple** — et le `POST` direct
   répond 403.
2. **Ajouter une compétence, valider, recharger** → elle figure dans les
   compétences du joueur, et le journal la marque `🛠️ Customisation`.
3. **Améliorer l'agilité** → l'affichage passe de `3+` à `2+`. Le scénario le
   plus important : il vérifie la table des directions de bout en bout.
4. **Améliorer jusqu'à la borne** → le bouton « Améliorer » se grise, et le
   `POST` forcé est refusé avec son motif.
5. **Ajouter une compétence déjà possédée** → refusée, motif affiché à côté de
   la compétence.
6. **Ajuster le prix** → la valeur du joueur change **et la valeur d'équipe
   suit**. Le pendant négatif compte autant : après une customisation de
   compétence, la TV **ne bouge pas**.
7. **Annuler** → le panier disparaît, le journal revient, et un rechargement
   ne rouvre pas le mode.
8. **Recharger en cours de saisie** → le panier est retrouvé intact, mode
   customisation rouvert.
9. **Prix sous zéro** → refusé.
10. **Panier périmé** — `updated_at` reculé de plus de 24 h : l'ouverture de la
    fiche retombe sur le journal, le panier a disparu, et le message d'abandon
    s'affiche.

Le scénario 6 est celui qui protège l'asymétrie de la phase 1 — la seule règle
de cette fonctionnalité qu'un lecteur de bonne foi prendrait pour un bug.

Entrée à ajouter à `tests/impact-map.toml` dans le même commit que le test.

---

## Règles métier (identifiées phase 7)

- **`ConcurrentWrite` n'est pas une erreur d'utilisateur.** Le panneau re-rendu
  porte l'état réel, sans message d'échec.
- **La projection se recalcule, elle ne s'incrémente pas** — insensible au
  rejeu et aux corrections de match.
- **Un panier périme après 24 h d'inactivité**, vérifié à l'ouverture de la
  fiche, avec message discret au retour.

## Points ouverts

Aucun. La conception est close — reste la phase 8, qui la découpe en cartes.
