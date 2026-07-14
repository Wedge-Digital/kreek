# Player report events — Phase 7 : Intégration

Pas de handler HTTP, pas de template Askama — feature 100% événementielle inter-BC.
Cette phase couvre : persistance, contrats d'événements finalisés (Phase 4, pas
documentée séparément — trop couplée à ce wiring pour justifier un fichier à part),
wiring publisher/listeners, et le choix des tests (pas d'E2E possible, justifié
plus bas).

---

## 1. `references` — nouvelle donnée : barème SPP

Fichier : `src/app/references/domain/port.rs` (trait `IReferenceRepository`), + implémentation dans le repository in-memory existant.

```rust
pub trait IReferenceRepository: Send + Sync {
    // ... méthodes existantes inchangées ...

    fn touchdown_spp(&self) -> u8;
    fn pass_spp(&self) -> u8;
    fn interception_spp(&self) -> u8;
    fn casualty_spp(&self) -> u8;
    fn mvp_spp(&self) -> u8;
}
```

Valeurs fixes (règle Blood Bowl standard), retournées en dur par l'implémentation — pas de nouvelle entrée JSON dans `assets/references/`, ce barème n'est ni roster-spécifique ni compétition-spécifique.

Méthodes nommées explicitement par type d'action (conformément à la demande), plutôt qu'une méthode générique paramétrée par un type d'action — cohérent avec le style `find_x`/`list_x` déjà en place sur ce trait, et évite un enum de mapping supplémentaire.

---

## 2. `players` — accès à `references` : pas de nouveau port

Correction par rapport à l'esquisse de la Phase 6 : `players` a déjà un précédent direct dans `team_created_listener.rs`, qui consomme `references::domain::port::IReferenceRepository` sans passer par un port ACL local. `players::context::init_listeners` reçoit déjà `refs: Arc<dyn IReferenceRepository>`. On réutilise ce même accès pour :
- `find_position_by_uid(roster_line_id)` (déjà existant) → stats de base (`player_stats_service::resolve_stats`, Phase 6)
- `touchdown_spp()`/`pass_spp()`/etc. (nouveaux, ci-dessus) → résolution du SPP par le nouveau listener, avant d'appeler la méthode domaine

Aucun nouvel `infrastructure/players/` requis.

---

## 3. Persistance — `players`

### `players_events` / `players_proj`

Fichier : `src/app/players/io/repository/player_repository.rs`

- `event_type_name()` et `player_and_team_id()` : ajout des 8 nouveaux variants (match exhaustif — ne compile pas tant qu'ils ne sont pas couverts).
- `upsert_player_projection()` : ajout des branches correspondantes.
  - `TouchdownScored`/`PassCompleted`/`InterceptionMade`/`CasualtyInflicted`/`MatchMvpNamed` → `UPDATE players_proj SET spp = spp + $spp_earned, version = version + 1 WHERE player_id = $1` (garde `players_proj.spp` synchronisé avec l'agrégat).
  - `FoulCommitted` → no-op sur la projection (rien d'utile à afficher en liste pour l'instant), juste `version = version + 1` pour rester cohérent.
  - `InjurySustained` → `UPDATE players_proj SET participation_status = $status, version = version + 1 WHERE player_id = $1` (nouvelle colonne, cf. migration ci-dessous). `$status` dérivé du même mapping que `apply()` (Commotion → inchangé, donc pas d'UPDATE de `participation_status` dans ce cas précis, juste `version += 1`).
  - `PlayerAvailabilityRestored` → `UPDATE players_proj SET participation_status = 'Available', version = version + 1 WHERE player_id = $1`.

**Choix de portée** : on ne projette **pas** `career_touchdowns`/`career_passes`/etc., `injuries`, `stat_adjustments` dans `players_proj` pour cette carte — aucun lecteur existant ou prévu à court terme n'en a besoin (la fiche joueur détaillée, seule consommatrice envisagée, n'est pas encore câblée). Ces données restent disponibles via l'agrégat complet (`IPlayerRepository::find_by_id`/`find_by_team_id`, déjà existants, aucune méthode nouvelle). Rebuildable depuis l'event store si on décide de les projeter plus tard — conforme à la règle CLAUDE.md sur les projections dérivées.

### Migration SQL (nouvelle)

```sql
ALTER TABLE players_proj
  ADD COLUMN participation_status TEXT NOT NULL DEFAULT 'Available';
```

### `find_by_team_id`

Déjà existant sur `IPlayerRepository` (`player_repository.rs:175-209`) — réutilisé tel quel par le nouveau listener `TeamMatchConcluded`, aucune méthode à ajouter.

---

## 4. Contrats d'événements (Phase 4 — finalisés ici)

Nouveau fichier : `src/app/shared_kernel/app_events/player_match_impact_app_events.rs`

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlayerMatchContextPayload {
    pub match_report_id:    String,
    pub round_id:           String,
    pub round_label:        String,
    pub opponent_team_id:   String,
    pub opponent_team_name: String,
    pub player_id:          String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum InjuryTypePayload {
    Commotion,
    Amoche,
    BlessureSerieuse,
    Sequel { stat: String },   // "Ma" | "St" | "Ag" | "Pa" | "Av"
    Mort,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PlayerMatchImpactAppEvent {
    PlayerPerformedTouchdown(PlayerMatchContextPayload),
    PlayerPerformedPass(PlayerMatchContextPayload),
    PlayerPerformedInterception(PlayerMatchContextPayload),
    PlayerPerformedCasualty(PlayerMatchContextPayload),
    PlayerPerformedMvp(PlayerMatchContextPayload),
    PlayerPerformedFoul(PlayerMatchContextPayload),
    PlayerInjured { context: PlayerMatchContextPayload, injury_type: InjuryTypePayload },
    TeamMatchConcluded { team_id: String, match_report_id: String },
}
```

Important : ce payload porte un `InjuryTypePayload` **structuré** (avec `stat` pour `Sequel`), distinct du `injury_label(&InjuryType) -> String` déjà utilisé par `MatchReportPublishedPayload` existant (qui, lui, perd l'info de stat — `"Sequel"` en texte brut, suffisant pour l'affichage humain de `MatchReportPublished` mais pas pour piloter `stat_adjustments` côté `players`). Les deux payloads coexistent, chacun avec son niveau de détail adapté à son consommateur.

`TeamMatchConcluded` est un variant de la **même** enum (pas un fichier séparé) — un seul point d'écoute app-event-bus à filtrer côté `players`, cohérent avec le fait que les deux proviennent du même publisher/moment.

---

## 5. `match_report` — publisher étendu

Fichier : `src/app/match_report/io/app_events/app_event_publisher.rs`

### Signature

```rust
pub fn match_report_app_event_publisher(
    event_bus: &EventBus,
    app_event_bus: EventBus,
    repo: Arc<dyn IMatchReportRepository>,
    competition_data: Arc<dyn ICompetitionDataPort>,   // nouveau
    team_data: Arc<dyn ITeamDataPort>,                  // nouveau
) {
```

Ports déjà existants (`ports.rs`), déjà utilisés ailleurs dans `match_report` pour l'affichage — seule leur injection dans **ce** publisher est nouvelle.

### Dans le handler `MatchReportPublished`, après confirmation de l'état `Published(p)`

1. Résoudre **une seule fois** (pas par action) :
   - `competition_data.find_round_context(&p.season_id, &p.round_id)` → `round_label`
   - `team_data.find_team_info(&p.home_team_id)` / `find_team_info(&p.away_team_id)` → noms des deux équipes
2. Construire le `PlayerMatchContextPayload` commun pour le côté home (opponent = away) et away (opponent = home).
3. Pour chaque action de `p.home_actions` puis `p.away_actions` :
   - Filtrer `ActionPlayer::Regular(player_id)` uniquement (BR1) — `ActionPlayer::Temp` ignoré.
   - Mapper `MatchActionType` → variant (`Touchdown`→`PlayerPerformedTouchdown`, `Passe`|`Lancer`→`PlayerPerformedPass` (BR2), `Interception`→`PlayerPerformedInterception`, `Sortie`→`PlayerPerformedCasualty`, `Mvp`→`PlayerPerformedMvp`, `Agression`→`PlayerPerformedFoul`, `Blesse{injury}`→`PlayerInjured{injury_type: map_injury(injury)}` avec mapping structuré complet, pas `injury_label()`).
   - `app_event_bus.send(...)`.
4. Émettre `TeamMatchConcluded{team_id: home_team_id, ...}` et `TeamMatchConcluded{team_id: away_team_id, ...}` — une fois chacun, indépendamment du nombre d'actions.

`build_published_payload` (existant, `MatchReportPublished` "classique") reste inchangé — les deux émissions cohabitent dans le même handler.

### Wiring — `context.rs` / `main.rs`

- `match_report::context::init_listeners()` gagne 2 paramètres (`competition_data`, `team_data`), transmis à `match_report_app_event_publisher`.
- `main.rs:116` (`match_report::context::init_listeners(&event_bus, &app_event_bus, pool.clone())`) : réutiliser les mêmes instances `Arc<dyn ICompetitionDataPort>`/`Arc<dyn ITeamDataPort>` déjà construites pour `MatchReportContext::new` juste avant dans `main.rs` — pas de nouvel adapter à écrire, juste un clone d'`Arc` supplémentaire au call site.

---

## 6. `players` — nouveaux listeners

### `src/app/players/io/app_events/player_match_impact_listener.rs` (nouveau)

- S'abonne à `app_event_bus`, désérialise en `PlayerMatchImpactAppEvent`, ignore `TeamMatchConcluded` (traité par l'autre listener) et tout ce qui n'est pas de ce type.
- Pour les 5 variants SPP (`PlayerPerformedTouchdown`/`Pass`/`Interception`/`Casualty`/`Mvp`) : résout le montant via `ref_repo.touchdown_spp()`/etc., construit `MatchContext` depuis le payload, `player.record_touchdown(context, SppEarned::try_new(amount).unwrap())` (le barème garantit `>= 1`, cf. BR4), event → `insert_player_event` + `upsert_player_projection` dans une tx (même pattern que `team_created_listener`).
- `PlayerPerformedFoul` : `player.record_foul(context)`, même persistance.
- `PlayerInjured` : `player.record_injury(context, injury_type)` (mapping `InjuryTypePayload` → `InjuryType` domaine), même persistance.
- `find_by_id(player_id)` requis avant chaque commande (charger l'agrégat pour connaître sa version courante) — même pattern que les autres listeners du projet.
- Joueur introuvable (`find_by_id` → `None`) : `tracing::warn!` + `continue`, comme `match_report_confirmed_listener`.

### `src/app/players/io/app_events/team_match_concluded_listener.rs` (nouveau)

- S'abonne à `app_event_bus`, filtre `PlayerMatchImpactAppEvent::TeamMatchConcluded`.
- `player_repo.find_by_team_id(&team_id)` (déjà existant), filtre `participation_status == MissingNextGame`, pour chacun : `player.restore_availability(match_report_id)`, persiste.
- No-op silencieux pour les joueurs non `MissingNextGame` (BR12).

### Wiring — `players/context.rs`

```rust
pub fn init_listeners(app_event_bus: &EventBus, pool: PgPool, refs: Arc<dyn IReferenceRepository>) {
    team_created_listener::init(app_event_bus, pool.clone(), refs.clone());
    player_match_impact_listener::init(app_event_bus, pool.clone(), refs);       // nouveau
    team_match_concluded_listener::init(app_event_bus, pool);                     // nouveau
}
```

Aucun nouveau paramètre à `init_listeners` — `refs` déjà présent est suffisant pour les deux nouveaux listeners.

---

## 7. Tests prévus

### Unitaires (domaine) — déjà listés en Phase 6, `player.rs`

### Domain service — déjà listé en Phase 6, `player_stats_service.rs`

### Intégration repository (vraie PgPool, pas de mock — cf. CLAUDE.md)

Dans `player_repository.rs` (module de tests existant à étendre) :

```rust
#[sqlx::test]
async fn append_touchdown_scored_credits_spp_in_projection(pool: PgPool) { /* ... */ }

#[sqlx::test]
async fn append_injury_sustained_updates_participation_status_in_projection(pool: PgPool) { /* ... */ }

#[sqlx::test]
async fn find_by_team_id_returns_only_missing_next_game_players_for_restoration(pool: PgPool) { /* ... */ }
```

### Pas de test E2E Playwright

Déviation assumée de la règle de couverture obligatoire (CLAUDE.md) : cette feature n'a **aucune surface HTML/HTMX/Alpine** — rien à piloter au navigateur, le déclencheur est un app event interne, pas une interaction utilisateur. La substitution proposée est un **test d'intégration bout-en-bout** (nouveau fichier, ex. `tests/integration/player_match_impact_test.rs` ou équivalent selon la convention de test d'intégration déjà en place dans le projet) qui : publie un `MatchReportPublished` factice sur le bus interne `match_report`, laisse les listeners réels tourner contre une vraie PgPool, et vérifie l'état final de l'agrégat `Player` (SPP crédité, compteurs incrémentés, statut correct) — sans navigateur, avec de vraies transactions et un vrai bus d'événements. À détailler en Phase 8 (cartes) si une carte dédiée est nécessaire pour ce test.

---

## 8. Résumé des fichiers créés/modifiés

| Fichier | Nature |
|---|---|
| `src/app/references/domain/port.rs` | Ajout 5 méthodes barème SPP à `IReferenceRepository` |
| `src/app/references/io/repository/*` (impl in-memory) | Implémentation des 5 méthodes (valeurs fixes) |
| `src/app/shared_kernel/app_events/player_match_impact_app_events.rs` | **Nouveau** — contrats Phase 4 |
| `src/app/match_report/io/app_events/app_event_publisher.rs` | Signature +2 params, nouvelle logique d'émission par action + `TeamMatchConcluded` |
| `src/app/match_report/context.rs` | `init_listeners()` +2 params |
| `src/main.rs` | Call site `match_report::context::init_listeners` mis à jour |
| `src/app/players/domain/match_impact.rs` | **Nouveau** (Phase 6) |
| `src/app/players/domain/player.rs` | Champs + `apply()` + méthodes de commande (Phase 6) |
| `src/app/players/domain/events.rs` | 8 nouveaux variants (Phase 6) |
| `src/app/players/io/repository/player_repository.rs` | `event_type_name`/`player_and_team_id`/`upsert_player_projection` étendus |
| `migrations/` | **Nouvelle migration** — `players_proj.participation_status` |
| `src/app/players/use_cases/player_stats_service.rs` | **Nouveau** (Phase 6, corrigé ici — pas de nouveau port) |
| `src/app/players/io/app_events/player_match_impact_listener.rs` | **Nouveau** |
| `src/app/players/io/app_events/team_match_concluded_listener.rs` | **Nouveau** |
| `src/app/players/context.rs` | `init_listeners()` enregistre les 2 nouveaux listeners |
| Test d'intégration bout-en-bout (nom exact à définir Phase 8) | **Nouveau** — remplace l'E2E Playwright, absent car pas de front |
