# Bandeau de la fiche équipe · Phase 7 : intégration

## 1. Persistance des événements

Les quatre événements neufs s'écrivent dans `team_event_store` en JSON, comme
les autres ; `type_name()` gagne un bras par événement. Aucun événement existant
ne change de forme : rien à relire autrement.

## 2. La projection `team_proj` — liste exhaustive

Le `_ => {}` de fin de `match` disparaît (`team_repository.rs`). Les 34
événements de l'équipe sont nommés et classés.

| Groupe | Événements | Projection |
|---|---|---|
| **les neufs** | les trois ouvertures | `game_phase` = la phase ouverte |
| | `ManualPhaseClosed` | `game_phase = 'ReadyToPlay'` |
| **jamais émis, mais ils touchent une colonne** | `TeamRenamed` → `team_name` ; `OffSeasonStarted`, `RetirementPhaseValidated` → `game_phase = 'OffSeason'` ; `OffSeasonCompleted` → comme `apply()` : `status = 'PendingEnrollment'`, `competition_id`, `season_id` et `game_phase` à `NULL` | **leur bras est écrit dès maintenant** (décision du 2026-10-03) : le jour où ils seront émis, la projection suivra |
| | `LogoChanged` → `logo_url` | **pas de bras ici** : c'est l'objet de la PR #11. Rangé dans le groupe sans effet, commentaire renvoyant à la PR — ne pas écrire deux fois le même bras |
| **sans effet sur `team_proj`** | `InducementsPaid`, `InducementsRefunded`, `InitialsChanged`, `JourneymanFielded`, `JourneymanRecruited`, `JourneymanWithdrawn`, `PlayerDismissed`, `PlayerNotReEngaged`, `PlayerReEngaged`, `PlayerRecruited`, `PlayerRetiredTemporarily`, `StaffDismissed`, `GamePhaseOverridden` | groupés, nommés un par un, commentaire disant pourquoi |

Un événement ajouté demain ne compile pas tant qu'il n'est pas classé : c'est la
fin de la famille de bugs des cartes 175 et 408. La PR #11 aura un conflit à
résoudre en rebasant, ce `match` changeant de forme.

## 3. Les listeners

`team_value_listener` et `phase_basket_purge_listener` appellent
`TeamDomainEvent::returns_to_ready_to_play()` ; leurs deux copies de la liste
disparaissent. `ManualPhaseClosed` déclenche ainsi, à la sortie d'une phase
manuelle, le recalcul de la valeur d'équipe et la purge des paniers.

## 4. Le publisher

Les quatre événements rejoignent le groupe explicite de ceux qui ne sortent pas
du BC — sans joker.

## 5. L'adapter de correction

`ref_team_data_adapter::is_team_in_player_improvement` ne répond oui qu'à une
phase `PlayerImprovement` **d'entrée `PostMatch`** : une phase de dépense
ouverte à la main ne rend aucun rapport corrigeable. Le pendant, côté écran, de
la garde du domaine (phase 6, point 7).

La question est posée au domaine, `Team::is_in_post_match_improvement()`, et non
recalculée dans l'adapter (décision du 2026-10-03, carte 576).

## 6. Les réponses HTTP

| Cas | Réponse |
|---|---|
| succès | `HX-Refresh: true` |
| refus du domaine (plus prête à jouer), motif refusé | **200**, `HX-Retarget` / `HX-Reselect` / `HX-Reswap` vers `#phase-override-foot`, le pied du panneau portant le message |
| phase inconnue | 400 |
| sans session, équipe introuvable, visiteur non admin | 401, 404, 403 — posés par `require_team_admin` |

## 7. Observabilité et `check-arch`

- Le use case est instrumenté ; les événements sont émis par le dépôt via
  `emettre`, comme les autres (axe 12).
- Axe 18 : les quatre événements sont émis — les ouvertures par le use case,
  `ManualPhaseClosed` par les trois sorties.
- Axe 5 : la projection est écrite dans la transaction de l'événement.

## 8. Les tests e2e

- L'admin ouvre chacune des trois phases depuis le bandeau ; l'équipe passe
  dans la phase, avec son bandeau habituel.
- Le coach dépense des SPP pendant une phase de dépense ouverte à la main,
  valide ; l'équipe revient prête à jouer sans passer par le recrutement, et sa
  valeur d'équipe intègre l'achat.
- Une sortie de renvois manuelle au-dessus du seuil ramène à prête à jouer, sans
  écran d'erreurs coûteuses.
- Un simple membre ne voit pas le bouton ; sa requête forgée reçoit 403.
- Une équipe qui n'est plus prête à jouer reçoit le message au pied du panneau.
- Pendant une phase manuelle, le dernier rapport de match n'est pas corrigeable.

## Règles métier

Question posée le 2026-10-03 — deux décisions à cette étape :

11. Les bras de projection de `TeamRenamed`, `OffSeasonStarted`,
    `OffSeasonCompleted` et `RetirementPhaseValidated` sont écrits maintenant,
    bien qu'ils ne soient pas encore émis.
12. `LogoChanged` reste à la PR #11.
13. Le bras de `OffSeasonCompleted` reprend `apply()` en entier, et non la seule
    phase (carte 576).
