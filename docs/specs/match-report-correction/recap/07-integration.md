# Phase 7 — Effets de bord · page Recap

## 1. Persistance

### Migrations

| Migration | Objet |
|---|---|
| index unique sur `ranking_lines (match_report_id, team_id)` | dette préexistante n°2 du README — rend le rejeu sûr et fait échouer bruyamment un double comptage |

**Aucune autre migration.** Les instantanés de compensation de `teams` et
`players` sont de l'**état dérivé**, reconstruit à l'hydratation depuis les
événements existants (phase 6). Aucun schéma d'événement n'évolue.

### Méthodes de repository

| Repository | Méthode | Statut |
|---|---|---|
| `IMatchReportRepository` | `append` | existante — réutilisée telle quelle |
| `ITeamRepository` | `append` | existante |
| `IPlayerRepository` | `append` | existante |
| `IRankingRepository` | `delete_lines_for_match(match_report_id)` | **nouvelle** |
| `competitions` | — | SQL direct dans le listener, comme l'existant |

### Projections — à mettre à jour dans la même transaction

Règle ES du CLAUDE.md : toute mise à jour de projection s'exécute dans la
transaction de l'append qui la déclenche.

| Projection | Effet de la compensation | Point de vigilance |
|---|---|---|
| `players_proj` | `spp` et `value_kpo` restaurés | `upsert_player_projection` doit traiter le nouvel événement de compensation — sinon la projection reste figée sur les valeurs post-match |
| `team_projection` | `game_phase` repasse à `MatchReporting`, trésorerie et fans restaurés | vérifier que `TeamRepository::append` couvre le nouvel événement |
| `competition_match_display_proj` | `match_status` → `in_progress`, scores et sorties à `NULL` | UPDATE à valeurs absolues, idempotent |
| `ranking_lines` | 2 lignes supprimées | `DELETE`, idempotent |

Les deux premières lignes sont les plus risquées : un événement de compensation
non traité par la fonction de projection **compile sans broncher** et laisse une
projection désynchronisée. À couvrir par un test d'intégration repository, pas
seulement unitaire.

## 2. Adapters — implémentation des deux méthodes de port

### `is_team_in_player_improvement`

`RefTeamDataAdapter` charge déjà l'agrégat via `ITeamRepository::find_by_id`. La
méthode est le miroir exact de l'`is_team_ready_to_play` voisine :

```rust
Ok(team.map(|t| t.game_phase == Some(GamePhase::PlayerImprovement)).unwrap_or(false))
```

Une équipe introuvable ou dissoute (`game_phase == None`) renvoie `false`, donc
bloque — conforme à la règle 16.

### `has_spent_spp_since_match`

`PlayerDataAdapter` ne dispose aujourd'hui que de
`IPlayerProjectionRepository` : la projection ne porte pas l'historique
nécessaire. La méthode interroge donc la table d'événements `players_events`
(`id BIGSERIAL` monotone, index sur `team_id`) :

```sql
SELECT EXISTS (
  SELECT 1 FROM players_events
  WHERE team_id = $1
    AND event_type IN ('PlayerSkillPurchased', 'PlayerStatIncreased')
    AND id > (
      SELECT MIN(id) FROM players_events
      WHERE team_id = $1
        AND event_type = 'MatchConcluded'
        AND payload -> 'MatchConcluded' -> 'context' ->> 'match_report_id' = $2
    )
)
```

**Point à confirmer à l'implémentation** : le chemin JSON dépend de la
représentation serde de `PlayerDomainEvent`, un enum *externally tagged* (aucun
`#[serde(tag = ...)]` sur le type). Le chemin ci-dessus suppose
`{"MatchConcluded": {...}}`. Si cette forme s'avère fragile, une variante
équivalente **sous le garde-fou « à chaud »** évite tout accès JSON :

```sql
AND id > COALESCE((SELECT MAX(id) FROM players_events
                   WHERE team_id = $1 AND event_type = 'MatchConcluded'), 0)
```

Elle est correcte parce que le match corrigible est nécessairement le dernier —
mais elle perd l'expressivité du paramètre `match_report_id`. Préférer la
première si le chemin JSON se vérifie.

Conséquence de câblage : `PlayerDataAdapter` a besoin d'un accès à la table
d'événements — soit une `PgPool`, soit une nouvelle méthode sur un port du BC
`players`. La seconde voie respecte mieux la souveraineté des données : le BC
`players` reste propriétaire de ses tables.

## 3. Événements et câblage

### Événements domaine

| BC | Événement | Émis par |
|---|---|---|
| `match_report` | `MatchReportUnpublished` | `MatchReportPublished::unpublish()` |
| `teams` | `PostMatchSequenceReverted` | `Team::revert_post_match_sequence()` |
| `players` | événement de compensation d'impact | `Player::revert_match_impact()` |

### App events

Émis par `match_report_app_event_publisher`, qui filtre déjà sur le type
d'événement domaine — il faut y ajouter la branche `MatchReportUnpublished` à
côté de `MatchReportPublished` :

| App event | Cardinalité |
|---|---|
| `MatchReportAppEvent::MatchReportUnpublished` | 1 |
| `PlayerMatchImpactAppEvent::TeamMatchImpactReverted` | 2 |

Après une dépublication, le publisher relit l'agrégat et le trouve en
`ReadyToPublish` (et non `Published`) — la fonction de relecture doit accepter
les deux états selon l'événement traité, sans quoi elle logue un `warn!` et
n'émet rien.

### Câblage des listeners

| BC | Fichier de câblage | Ajout |
|---|---|---|
| `competitions` | `context.rs` → `init_listeners` | `match_report_unpublished_listener::init(...)` |
| `ranking` | `context.rs` → `init_listeners` | `match_report_unpublished_listener::init(...)` |
| `teams` | `context.rs` → `init_listeners` | `match_report_unpublished_listener::init(...)` |
| `players` | — | aucun : traité dans `player_match_impact_listener` existant |

**Aucun changement dans `main.rs`** : les trois `init_listeners` sont déjà
appelés, et les deux adapters de port sont déjà instanciés et injectés.

## 4. Handlers

### `post_unpublish` — nouveau

```rust
pub async fn post_unpublish(
    auth_session: AuthSession,
    Path((space_id, match_report_id)): Path<(String, String)>,
    State(state): State<AppState>,
) -> impl IntoResponse
```

| Cas | Réponse |
|---|---|
| non connecté | `401` |
| `match_report_id` invalide | `400` |
| non autorisé (`is_authorized`) | `403` |
| succès | `HX-Refresh: true` |
| `NotEligible(_)` | `HX-Refresh: true` — la page se recharge sur l'état bloqué avec la raison à jour (règle 9a) |
| `NotFound` / `NotPublished` | `404` |
| `Repository(_)` | `500` |

`NotEligible` renvoie un rafraîchissement et non une erreur : c'est la
traduction HTTP de la règle 9a. Le coach voit la raison à jour, pas une page
d'erreur.

Découpage pour la règle des 20 lignes : `post_unpublish` délègue l'autorisation
à une fonction dédiée et la traduction du résultat à une seconde.

### `get_recap` — modifié

Alimente deux champs supplémentaires de `RecapTemplate` :

- `correction: Option<CorrectionZoneVm>` — `Some(_)` si le rapport est publié,
  construit par `build_correction_zone` après appel du domain service
- `under_correction: bool` — `!is_published && was_published_before`

Les 4 appels de port du garde-fou s'ajoutent au `tokio::join!` existant.

### `post_publish` — modifié (carte prérequis)

Ajout de l'appel à `is_authorized()`, absent aujourd'hui. Dette préexistante
n°1 du README.

## 5. Templates et CSS

| Fichier | Modification |
|---|---|
| `recap.html` | bandeau conditionnel sur `under_correction`, avant `ms-cta-row` ; zone de correction après, dans la branche `is_published` |
| `match-report-recap.css` | classes `.ms-correct-*` et `.ms-unpublished-banner`, reprises de la maquette |

Aucun nouveau template : la zone vit dans le template de page existant (phase 2,
pas de widget). Aucun style inline — interdits par CLAUDE.md.

VMs consommés : `CorrectionZoneVm` (`can_correct`, `blocked_reason`,
`unpublish_url`). Le template n'assemble aucun message : `blocked_reason` arrive
sous forme de phrase complète.

## 6. Tests E2E prévus

Fichier : `tests/e2e/test_match_report_correction.py`

| # | Scénario | Règles couvertes |
|---|---|---|
| 1 | Rapport publié, garde-fou passant → le bouton est actif ; correction → retour en `ReadyToPublish` avec le bandeau | — |
| 2 | L'adversaire achète une compétence → bouton désactivé, message nommant **son** équipe | 2, 3 |
| 3 | Une équipe valide sa phase d'amélioration → bouton désactivé, message adapté | 1, 3 |
| 4 | Correction puis re-publication avec un score modifié → le classement reflète le **nouveau** score, sans ligne résiduelle de l'ancien | 8, 11 |
| 5 | Correction → le match repasse « en cours » dans les résultats de compétition ; re-publication → « terminé » | — |
| 6 | Correction → trésorerie et fans de l'équipe restaurés sur sa fiche | 14 |
| 7 | Deux corrections successives sur le même rapport aboutissent | 8 |
| 8 | Le bandeau reste visible après modification d'une action, donc après passage par `PreMatch` | phase 4 |

Le scénario 8 est celui qu'un test unitaire ne peut pas attraper : c'est le
parcours réel `ReadyToPublish → PreMatch → ReadyToPublish` qui le met en
défaut si le drapeau ne se propage pas.

Les tests décisifs sur l'écrêtage des fans à 0 et 20 (règle 14) restent
**unitaires** — les provoquer en navigateur demanderait de construire un
historique de fans très long pour un gain de confiance nul.

## 7. Ordre d'implémentation contraint

Trois dépendances à respecter en phase 8 :

1. **Carte 225** (disponibilité des joueurs blessés) avant toute compensation
   côté `players` — elle fixe la définition de « avant » pour la règle 15
2. **Autorisation sur `post_publish`** avant le handler de correction — la
   règle 4 aligne les droits, l'un hérite de l'autre
3. **Index unique sur `ranking_lines`** avant la compensation `ranking`
