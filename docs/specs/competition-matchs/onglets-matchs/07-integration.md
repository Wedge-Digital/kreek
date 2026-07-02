# Onglets Résultats & Calendrier — Phase 7 : Intégration

---

## Migration SQL

### Nouvelle table `competition_match_display_proj`

```sql
CREATE TABLE competition_match_display_proj (
    pairing_id          TEXT PRIMARY KEY,
    season_id           TEXT NOT NULL,
    round_id            TEXT NOT NULL,
    round_name          TEXT NOT NULL,
    round_position      INTEGER NOT NULL,
    round_date_start    TEXT,
    round_date_end      TEXT,
    round_day_type      TEXT NOT NULL,  -- fixed_date | time_frame | rest
    home_team_id        TEXT NOT NULL,
    home_team_name      TEXT NOT NULL,
    home_roster_name    TEXT NOT NULL,
    home_coach_name     TEXT NOT NULL,
    home_logo_url       TEXT,
    home_initials       TEXT NOT NULL,
    away_team_id        TEXT NOT NULL,
    away_team_name      TEXT NOT NULL,
    away_roster_name    TEXT NOT NULL,
    away_coach_name     TEXT NOT NULL,
    away_logo_url       TEXT,
    away_initials       TEXT NOT NULL,
    match_status        TEXT NOT NULL DEFAULT 'upcoming',
    home_score          INTEGER,
    away_score          INTEGER,
    home_casualties     INTEGER,
    away_casualties     INTEGER,
    match_report_id     TEXT,
    match_report_url    TEXT
);

CREATE INDEX idx_cmdp_season_status   ON competition_match_display_proj (season_id, match_status);
CREATE INDEX idx_cmdp_season_position ON competition_match_display_proj (season_id, round_position);
```

---

## Événements — alimentation de la projection

### BC competitions — `PairingCreated` (event domaine enrichi)

**Quand** : à la création d'un pairing par `generate_pairings` / `generate_all_pairings`.

**Action** : INSERT dans `competition_match_display_proj` avec `match_status = 'upcoming'`.

L'event doit être émis **dans la même transaction** que l'INSERT en projection (cf. règle event sourcing CLAUDE.md).

### BC competitions — `PairingDeleted` (event domaine existant)

**Action** : DELETE FROM `competition_match_display_proj` WHERE `pairing_id = event.pairing_id`.

Même transaction que la suppression de l'event dans l'event store.

### BC competitions — listener sur `MatchReportConfirmed` (app event BC match_report)

**Trigger** : `MatchReportConfirmed` reçu depuis le BC `match_report` (selection confirmée = rapport démarré).

**Action** : UPDATE `competition_match_display_proj` SET `match_status = 'in_progress'`, `match_report_id = ...`, `match_report_url = ...` WHERE `pairing_id = event.pairing_id`.

**Prérequis** : `MatchReportConfirmed` doit être enrichi avec `pairing_id: Option<String>` (actuellement absent de l'app event). Ce champ est déjà disponible dans `MatchReportCreated` domain event et dans `create_match_report_use_case`.

Fichier à créer : `src/app/competitions/io/app_events/match_report_confirmed_listener.rs`

### BC competitions — listener sur `MatchReportFinalized` (app event à créer)

**Trigger** : futur app event émis par le BC `match_report` quand le rapport est finalisé (dernière étape non encore implémentée).

**Action** : UPDATE `competition_match_display_proj` SET `match_status = 'completed'`, `home_score`, `away_score`, `home_casualties`, `away_casualties` WHERE `pairing_id = event.pairing_id`.

> Note : cet event n'existe pas encore. La colonne `match_status = 'completed'` ne sera alimentée qu'à l'implémentation de la finalisation du rapport de match. Les onglets Résultats fonctionneront avec `in_progress` en attendant.

---

## Requêtes SQL

### `list_resultats.sql`

```sql
SELECT *
FROM competition_match_display_proj
WHERE season_id = $1
  AND match_status IN ('in_progress', 'completed')
  AND ($2::integer IS NULL OR round_position < $2)
ORDER BY round_position DESC
LIMIT 500  -- max pairings pour 3 journées (sécurité)
```

Le groupement par journée (3 max) est fait côté Rust après fetch.

### `list_calendrier.sql`

```sql
SELECT *
FROM competition_match_display_proj
WHERE season_id = $1
  AND match_status = 'upcoming'
  AND ($2::integer IS NULL OR round_position > $2)
ORDER BY round_position ASC
LIMIT 500
```

---

## Handlers

### `resultats_tab_controller::get_resultats_tab`

```
GET /spaces/:space_id/competitions/:cid/seasons/:sid/resultats?cursor=:journee_id

Extracteurs : Path(space_id, cid, sid), Query(TabCursorQuery), State, HeaderMap
Retour HTMX  : ResultatsTabTemplate { is_initial: cursor.is_none(), journees, next_cursor }
Retour direct : full page (fallback navigation directe) via load_page_base()
```

### `calendrier_tab_controller::get_calendrier_tab`

```
GET /spaces/:space_id/competitions/:cid/seasons/:sid/calendrier?cursor=:journee_id

Même structure que resultats_tab_controller.
```

---

## Templates

### `competition-tab-resultats.html`

```html
{% if is_initial %}<div id="resultats-list">{% endif %}

{% for journee in journees %}
<div class="section-block card-shadow">
  <div class="section-block-header">
    <span class="section-label">{{ journee.label }}</span>
    <span class="section-sub">{{ journee.matches.len() }} match…</span>
  </div>
  {% for m in journee.matches %}
  <div class="match-row …">
    {# côté domicile #}
    {# score / badge statut #}
    {# côté visiteur #}
  </div>
  {% endfor %}
</div>
{% endfor %}

{% if let Some(cursor) = next_cursor %}
<div hx-get="?cursor={{ cursor }}"
     hx-trigger="intersect once"
     hx-target="#resultats-list"
     hx-swap="beforeend"
     class="scroll-sentinel">Chargement…</div>
{% endif %}

{% if is_initial %}</div>{% endif %}
```

### `competition-tab-calendrier.html`

Structure identique, avec `date_range` dans le header de journée et sans bloc score.

### `competition-detail.html` (modifications)

- Remplacer l'onglet "Matchs" par "Résultats" et "Calendrier"
- Ajouter les conteneurs lazy-load :

```html
<div id="resultats-list"
     hx-get="{{ routes.competitions.resultats_tab(space_id, cid, sid) }}"
     hx-trigger="click from:#tab-btn-resultats once"
     hx-swap="innerHTML">
</div>

<div id="calendrier-list"
     hx-get="{{ routes.competitions.calendrier_tab(space_id, cid, sid) }}"
     hx-trigger="click from:#tab-btn-calendrier once"
     hx-swap="innerHTML">
</div>
```

---

## Tests E2E (Playwright)

| Scénario | Vérification |
|---|---|
| Clic sur onglet Résultats | Fragment chargé, journées affichées |
| Scroll jusqu'au sentinel | 3 nouvelles journées ajoutées |
| Fin du scroll (plus de journées) | Sentinel disparu, pas de rechargement |
| Match `in_progress` | Badge "En cours de saisie" + lien rapport visible |
| Match `completed` | Score affiché, pas de badge |
| Clic sur onglet Calendrier | Fragment chargé, journées à venir affichées |
| Scroll Calendrier | 3 nouvelles journées futures ajoutées |
| Navigation directe sur URL Résultats | Full page rendue avec onglet actif |
| Logos présents / absents | Image ou initiales selon `home_logo_url` |
