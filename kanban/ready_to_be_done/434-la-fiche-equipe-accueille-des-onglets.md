# La fiche équipe accueille des onglets

**Ordre :** 1 · **Dépend de :** rien
**Conception :** `docs/specs/tresorerie-equipe/onglet-tresorerie/02-front.md`
et `07-integration.md`

## Objectif

Donner à la fiche équipe un mécanisme d'onglets. **Aucun changement visible** :
« Joueurs & Staff » devient le premier onglet d'un aiguillage qui n'existait
pas, et l'écran reste exactement le même.

## Le constat

```html
<!-- teams-team-detail.html:145 -->
<div class="tabs team-tabs">
  <div class="tab active">Joueurs &amp; Staff</div>
  <div class="tab">Matchs</div>
  <div class="tab">Trésorerie</div>
</div>
```

**Trois `<div>` inertes.** Aucun `hx-get`, aucun lien, aucune route. La fiche
équipe n'a qu'une route, `TEAM_DETAIL`, et tout ce qui suit les onglets — le
widget joueurs et le panneau staff — est le contenu de « Joueurs & Staff » sans
que rien ne le dise.

## Conception

### Le conteneur

Tout ce qui suit les onglets — `#players-widget` et `.staff-panel` — entre dans :

```html
<div id="team-tab-content">…</div>
```

**Copier-coller, pas réécriture** (règle 5 du `CLAUDE.md`). Le bloc devient le
fragment de l'onglet « Joueurs & Staff », dans son propre gabarit
`teams-squad-tab.html`, inclus par la page.

### L'aiguillage

Sur le patron d'`admin_page.rs`, déjà en place :

```rust
pub async fn team_detail(…)          // active_tab = "squad"
pub async fn team_page_treasury(…)   // active_tab = "treasury"

let content = match active_tab {
    "treasury" => …,     // vide pour l'instant — la carte 436 le remplit
    _          => render_squad(…),
};
```

### La route

```rust
TEAM_TREASURY  "/app/{space_id}/teams/{team_id}/tresorerie"
```

Montée en `get(team_page_treasury)`. `space_scope` la couvre sans rien ajouter :
elle porte `{team_id}`, dont `teams` déclare déjà le résolveur
(`infrastructure/teams/space_ownership.rs`).

### Les onglets

« Joueurs & Staff » devient un `<a>` porteur de `hx-get` / `hx-target` /
`hx-push-url`, sur le modèle d'`admin-page.html:19`.

**« Trésorerie » reste un `<div>` inerte** — la carte 436 le câble. **« Matchs »
reste inerte définitivement** : hors périmètre, et une route qui répond « rien »
se lirait comme une panne.

`hx-swap="innerHTML"` sur `#team-tab-content` : le fragment est le **contenu**
du conteneur, jamais le conteneur lui-même. C'est la forme que le `CLAUDE.md`
exige — l'erreur qu'il proscrit est le fragment qui répète l'`id` de sa cible.

### Le style

**Rien à ajouter.** `.tabs` et `.tab` existent dans `pages/team-page.css` —
c'est ce qui fait que les trois `<div>` s'affichent correctement aujourd'hui.
Un `<a>` apporte son `cursor: pointer` de lui-même.

## Ce que la carte ne fait pas

- **Elle ne montre pas de trésorerie.** L'onglet reste inerte.
- **Elle ne câble pas « Matchs ».**
- **Elle ne change aucun rendu.** À l'écran, avant et après sont identiques.

## Tests

- **Unitaire** : l'aiguillage rend la branche `squad` par défaut, et la branche
  `treasury` sur `active_tab = "treasury"`.
- **E2E** : la fiche équipe s'affiche comme avant — le widget joueurs charge, le
  panneau staff est là. C'est une carte de refactoring : le test qui compte est
  celui qui prouve que rien n'a bougé.

`tests/e2e/test_team_detail_state_banner.py` doit rester vert sans modification.

## Checklist

- [ ] `#team-tab-content` et `teams-squad-tab.html` par copier-coller
- [ ] `TEAM_TREASURY` + `.route(...)` + `team_page_treasury`
- [ ] L'aiguillage `match active_tab`
- [ ] « Joueurs & Staff » en `<a>` htmx ; « Trésorerie » et « Matchs » inchangés
- [ ] `make lint && make test && make check-arch && make e2e`
