# Player detail page — Phases 2 à 7 : Conception

Câblage de la maquette `assets/rawpages/html/app-player-detail-readonly.html` sur
données réelles, à la place de la page de debug utilisée jusqu'ici comme cible
du clic sur une ligne du tableau roster.

## Périmètre

**Dans le scope** : en-tête (identité, stats, compétences), résumé de carrière,
portefeuille SPP (gagnés/dépensés/réserve), historique de matchs, journal des
évolutions (skills acquises à la construction d'équipe).

**Hors scope** (features à part, boutons présents mais non câblés) :
- Mode Customisation (JS de la maquette non porté)
- Page de dépense de SPP en saison (`app-player-detail.html`)

---

## Phase 2 — Architecture front

Page 100% lecture seule, aucune section interactive indépendante (pas de
mutation, pas de widget à endpoint propre) — ne remplit pas les critères du
pattern « page d'assemblage à widgets » (CLAUDE.md). **Un seul handler, un
seul template**, sur le modèle de `team_detail.rs`.

## Phase 3 — Architecture back

| Fichier | Nature |
|---|---|
| `players/io/web/player_detail_controller.rs` | **Nouveau** — handler + VMs |
| `players/io/web/templates/player-detail.html` | **Nouveau** — copié-adapté de la maquette |
| `players/use_cases/match_history_service.rs` | **Nouveau** — regroupement des events par match |
| `players/domain/match_impact.rs` | `MatchesPlayedCount` |
| `players/domain/events.rs` | Variant `MatchConcluded` |
| `players/domain/player.rs` | Champ, `apply()`, méthode de commande |
| `shared_kernel/app_events/player_match_impact_app_events.rs` | `TeamMatchConcluded` enrichi |
| `match_report/io/app_events/app_event_publisher.rs` | Émission enrichie |
| `players/io/app_events/team_match_concluded_listener.rs` | Restructuré |
| `players/io/repository/player_repository.rs` | `MatchConcluded` + `find_events_by_id` |
| `players/routes.rs`/`router.rs` | Route `player_detail` |

Vérification des droits (bouton Customiser) faite directement dans le handler
via `AppState` (`teams`/`competitions`/`spaces`), même pattern que
`competitions/io/web/admin/admin_page.rs` — pas de nouveau port ACL.

---

## Phase 4 — Contrats (VMs)

```rust
pub struct PlayerDetailVm {
    pub player_id: String, pub team_id: String, pub team_name: String,
    pub name: String, pub jersey: Option<i16>, pub position_name: String,
    pub ma: u8, pub st: u8, pub ag: u8, pub pa: u8, pub av: u8,
    pub base_skills: Vec<String>, pub acquired_skills: Vec<String>,
    pub value_formatted: String,
    pub spp_earned: u32, pub spp_spent: u32, pub spp_reserve: u32, pub spp_percent: u8,
    pub matches_played: u16,
    pub career_touchdowns: u16, pub career_passes: u16, pub career_interceptions: u16,
    pub career_casualties: u16, pub career_mvps: u16,
    pub can_customise: bool,
    pub match_history: Vec<MatchHistoryCardVm>,
    pub evolution_log: Vec<EvolutionLogRowVm>,
}
pub struct MatchHistoryCardVm {
    pub opponent_name: String, pub round_label: String,
    pub result_label: String, pub result_css: String,
    pub team_score: u8, pub opponent_score: u8,
    pub actions: Vec<MatchActionLineVm>, pub subtotal_spp: u32,
}
pub struct MatchActionLineVm { pub icon: String, pub label: String, pub spp: Option<u32> }
pub struct EvolutionLogRowVm { pub label: String, pub mode_label: String, pub cost: String, pub value: String, pub origin: String }
```

`spp_spent` = Σ `acquired_skills[].spp_cost` ; `spp_reserve` = `spp_earned - spp_spent` ;
`spp_percent` = `spp_spent * 100 / spp_earned` (0 si `spp_earned == 0`).

`evolution_log[].origin` = toujours `"Compétence initiale bonus"` pour l'instant
(seule source existante — la dépense de SPP en saison, hors scope, introduira
une origine différente le jour où elle existera).

Icônes/libellés par type d'action :

| Kind | Icône | Libellé |
|---|---|---|
| Touchdown | 🏈 | Touchdown |
| Pass | 🎯 | Passe réussie |
| Interception | 🛡️ | Interception |
| Casualty | 🩸 | Sortie infligée |
| Mvp | ⭐ | MVP |
| Foul | 🟨 | Faute |
| Injury | 🤕 | Blessure |

`result_label`/`result_css` dérivés par comparaison des scores (Victoire/vert,
Défaite/rouge, Nul/gris) — même règle que `build_team_banner`
(`match_report/io/web/builders.rs`), une seule source de vérité, rien de stocké.

---

## Phase 5 — Use case : reconstruction de l'historique

`players/use_cases/match_history_service.rs::build_match_history(events: &[PlayerDomainEvent]) -> Vec<MatchHistoryEntry>` —
regroupe les events bruts d'un joueur par `context.match_report_id` : l'event
`MatchConcluded` fournit l'en-tête (adversaire, journée, scores), les events
d'action du même match fournissent les lignes de détail. Tolérant à l'ordre
relatif action/`MatchConcluded` (les actions peuvent arriver avant l'event de
fin de match dans le flux d'émission du publisher). Retourné du plus récent au
plus ancien.

Rien n'est stocké en plus sur l'agrégat — reconstruction entièrement à la
lecture, conforme à la décision prise lors de la conception du domaine
match-impact.

---

## Phase 6 — Domaine

Nouveau champ `Player.matches_played: MatchesPlayedCount`. Nouveau domain event :

```rust
MatchConcluded { player_id, team_id, context, team_score: u8, opponent_score: u8 }
```

`apply()` : incrémente uniquement `matches_played`. Aucun résultat stocké
(dérivé à l'affichage, Phase 4).

---

## Phase 7 — Intégration

- **`TeamMatchConcluded`** (app event, shared_kernel) enrichi : `round_id`,
  `round_label`, `opponent_team_id`, `opponent_team_name`, `team_score`,
  `opponent_score` — toutes ces données sont déjà disponibles dans le scope où
  l'event est émis (`match_report_app_event_publisher::publish_player_impact_events`),
  aucun nouvel appel de port.
- **`team_match_concluded_listener.rs`** restructuré : pour **chaque** joueur
  de l'équipe → `MatchConcluded` (toujours) ; en plus, `PlayerAvailabilityRestored`
  si le joueur était `MissingNextGame`. Deux appends possibles par joueur
  (versions n+1 puis n+2), test du statut fait sur l'état chargé initialement.
- **`IPlayerRepository::find_events_by_id(player_id) -> Vec<PlayerDomainEvent>`**
  (nouveau) — events bruts, nécessaires à `match_history_service` (`find_by_id`
  ne renvoie que l'agrégat hydraté final).
- **Routing** : nouvelle route `player_detail`, le clic sur une ligne du
  tableau roster (`player-table-fragment.html`) pointe désormais vers cette
  page au lieu de `player_debug` (qui reste disponible comme outil de dev).
- **Tests E2E** : contrairement à la feature « player match impact » (100%
  backend, dérogation E2E assumée), cette page a une vraie surface HTML — la
  couverture E2E Playwright standard s'applique normalement ici (carte 168).
