# La fiche équipe accueille des onglets

**Épic :** E06 — La fiche d'équipe complétée · **Ordre :** 1 · **Dépend de :** rien
**Dont dépendent :** 436 (trésorerie), **477** (matchs), et par elle la 478
**Conception :** `docs/specs/fiche-equipe-onglets/README.md`

> **Trois cartes attendent ce mécanisme**, dont une d'un autre chantier —
> l'onglet Matchs, dont le contenu est servi par `competitions`. C'est la
> première carte de E06 à livrer, et rien ne se branche avant elle.

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

Sur le patron d'`admin_page.rs`, déjà en place (son `match active_tab`, ligne
209) :

```rust
pub async fn team_detail(…)          // active_tab = "squad"
pub async fn team_page_treasury(…)   // active_tab = "treasury"

let content = match active_tab {
    "treasury" => …,     // vide pour l'instant — la carte 436 le remplit
    _          => render_squad(…),
};
```

**Le `_` rend « Joueurs & Staff », il n'échoue pas.** Un `active_tab` inconnu
vient d'une URL tapée à la main ; répondre `404` sur une page qui existe serait
pire que d'afficher l'onglet par défaut.

### La route

```rust
TEAM_TREASURY  "/app/{space_id}/teams/{team_id}/tresorerie"
```

Montée en `get(team_page_treasury)`. `space_scope` la couvre sans rien ajouter :
elle porte `{team_id}`, dont `teams` déclare déjà le résolveur
(`infrastructure/teams/space_ownership.rs`).

`TEAM_MATCHES` suivra le même moule, posée par la 477 — **pas par celle-ci** :
une route montée sans contenu répond « rien ».

**Aucune route de fragment séparée.** C'est la route de page que le `hx-get`
appelle, et le handler distingue les deux usages à l'en-tête `HX-Request`, comme
l'administration de compétition. Une seconde route doublerait la surface pour la
même réponse.

### Les onglets

« Joueurs & Staff » devient un `<a>` porteur de `hx-get` / `hx-target` /
`hx-push-url`, sur le modèle d'`admin-page.html:19`.

**« Trésorerie » et « Matchs » restent des `<div>` inertes** — la 436 câble le
premier, la **477** le second.

La règle est qu'**un onglet ne devient cliquable que lorsque son contenu
existe** : une route qui répond « rien » se lit comme une panne. Chaque carte
câble donc **son** onglet ; celle-ci pose le mécanisme et n'en câble qu'un.

> Une version antérieure de cette carte disait « **Matchs** reste inerte
> **définitivement** ». La seconde moitié du motif reste vraie, le
> « définitivement » ne l'est plus — la 477 câble cet onglet.

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
