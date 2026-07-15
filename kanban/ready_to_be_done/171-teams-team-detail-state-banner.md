# BC `teams` — Bandeau d'état contextuel sur la page de détail d'équipe

**Priorité : haute**
**Dépend de :** `169-teams-domain-dismissals-target-match-report-id.md`, `170-teams-validate-phase-use-cases.md`
**Contexte :** `teams` — IO web, template

## Objectif

Afficher le bandeau d'état contextuel maquetté dans
`assets/rawpages/html/app-team-detail.html` (Phase 1, déjà validée) sur la
page de détail d'équipe réelle, avec ses actions. Spec complète :
`docs/specs/team-state-management/team-detail/02-07-conception.md` (Phases 2, 4, 7).

---

## Conception

### VM (`io/web/team_detail.rs`)

```rust
pub enum BannerCtaVm {
    Print,
    Navigate { label: String, href: String },
    Mutate   { label: String, post_url: String, outline: bool },
}

pub struct BannerVm {
    pub css_variant: String,   // "pending" | "ready" | "phase"
    pub icon: String,
    pub title: String,         // partie <strong>
    pub detail: String,
    pub ctas: Vec<BannerCtaVm>,
}
```

`BannerVm::from_domain(team: &Team, space_id: &str, app_routes: &AppRoutes) -> Option<BannerVm>`
— constructeur co-localisé (pur domaine + routes), un match sur
`(participation_status, game_phase)` :

| État | `css_variant` | CTA(s) |
|---|---|---|
| `PendingEnrollment` | `pending` | — |
| `Enrolled` + `ReadyToPlay` | `ready` | `Print` |
| `Enrolled` + `MatchReporting` | `phase` | `Navigate` → `match_report.edit_match_report(space_id, current_match_report_id)` |
| `Enrolled` + `PlayerImprovement` | `phase` | `Mutate` "Évolutions terminées" → validate-improvement-phase |
| `Enrolled` + `Recruitment` | `phase` | `Mutate` "Terminer les achats" → validate-recruitment-phase |
| `Enrolled` + `Dismissals` | `phase` | `Mutate` "Valider les renvois" → validate-dismissals-phase |
| tout le reste (`TemporaryRetirement`, `OffSeason`, `Dismissed`, `Rejected`, phase `None`) | — | `None` (pas de bandeau) |

`TeamDetailVm` gagne un champ `pub banner: Option<BannerVm>`, peuplé dans
`TeamDetailVm::from()`.

### Template (`teams-team-detail.html`)

Bloc conditionnel entre `.team-header` et `.tabs` :

```html
{% if let Some(banner) = vm.banner %}
<div class="state-banner state-banner--{{ banner.css_variant }}">
  <span class="state-banner-icon">{{ banner.icon }}</span>
  <span class="state-banner-text"><strong>{{ banner.title }}</strong> {{ banner.detail }}</span>
  {% for cta in banner.ctas %}
    {% match cta %}
      {% when BannerCtaVm::Print %}
        <button class="state-banner-cta state-banner-cta--outline" onclick="window.print()">Imprimer en PDF</button>
      {% when BannerCtaVm::Navigate { label, href } %}
        <a class="state-banner-cta" hx-get="{{ href }}" hx-target="#app-content" hx-select="#app-content" hx-swap="innerHTML" hx-push-url="true">{{ label }}</a>
      {% when BannerCtaVm::Mutate { label, post_url, outline } %}
        <button class="state-banner-cta{% if *outline %} state-banner-cta--outline{% endif %}" hx-post="{{ post_url }}">{{ label }}</button>
    {% endmatch %}
  {% endfor %}
</div>
{% endif %}
```

### CSS (`assets/static/css/pages/app-team-detail.css`)

Classes `.state-banner*` reprises telles quelles de
`assets/rawpages/html/app-team-detail.html` (lignes 98-120) — pas de style
inline.

---

## Checklist

- [ ] `BannerCtaVm` + `BannerVm` + `BannerVm::from_domain()`
- [ ] `TeamDetailVm.banner: Option<BannerVm>` peuplé dans `TeamDetailVm::from()`
- [ ] Bloc `state-banner` ajouté au template, une branche par variant de CTA
- [ ] CSS `.state-banner*` ajouté à `app-team-detail.css`
- [ ] Vérification manuelle des 4 états actionnables + 1 état informatif + absence de bandeau sur les états non couverts
