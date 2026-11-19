# Le bloc de match devient un composant

**Épic :** E06 — La fiche d'équipe complétée · **Ordre :** 1 · **Dépend de :** rien
**Conception :** `docs/specs/matchs-d-une-equipe/README.md`

## Objectif

Rendre réutilisable le bloc de match de l'onglet Résultats — gabarit **et**
feuille — sans changer une ligne de ce qu'il affiche aujourd'hui.

**Aucun écran ne change.** C'est une carte de préparation, et la seule du
chantier qui touche une page qui marche.

## Le gabarit

```
competitions/io/web/templates/
├── match-widget.html                  ← extrait, prend `m: MatchResultatVm`
└── competition-tab-resultats.html     ← l'inclut, garde ses groupes et son curseur
```

60 lignes à déplacer — les deux côtés symétriques, le bloc de score, le badge
« En cours de saisie », le lien de rapport conditionnel.

**Copier-coller, pas réécriture** (règle 5 du `CLAUDE.md`). Ces lignes marchent,
avec leurs `{% if let Some %}` et leurs `unwrap_or(0)` ; les retaper de mémoire
est le seul moyen d'y introduire un défaut.

## La feuille — la partie qui demande de l'attention

Les 93 règles `.match-widget`, `.matches-list`, `.match-side`, `.match-score`,
`.match-team`, `.match-cas`, `.match-status-badge` vivent dans
`pages/competition-detail.css`, **toutes préfixées `.competition-detail`** :

```css
.competition-detail .match-widget,
.competition-detail.match-widget { … }
```

Sur une autre page elles ne s'appliqueraient **pas du tout** : le bloc
s'afficherait nu. Et retirer simplement le préfixe ferait crier
`tests/e2e/visual/debordements.py`, qui pose exactement cette question — *ce
sélecteur trouve-t-il du markup sur une page qui ne chargeait pas sa feuille ?*

**Elles deviennent `components/match-widget.css`**, sans préfixe de page. Les
feuilles de `components/` sont globales par construction pour ce contrôle : le
débordement disparaît de lui-même.

C'est le même geste que les teintes de compétence de la carte 469 — un
sélecteur partagé entre deux pages n'a pas sa place dans la feuille de l'une
des deux.

## Deux champs neufs sur le VM, tous deux `Option`

```rust
pub struct MatchResultatVm {
    …,
    pub round_label: Option<String>,        // « Journée 3 »
    pub outcome:     Option<MatchOutcome>,  // V / N / D, du point de vue d'une équipe
}

pub enum MatchOutcome { Win, Draw, Loss }
```

**Les deux valent `None` sur l'onglet compétition**, et le gabarit ne rend rien
dans ce cas — d'où l'absence de changement visible.

`round_label` y est `None` parce que l'en-tête de groupe le dit déjà.
`outcome` y est `None` parce que la question n'a pas de sens : sur une page de
compétition il n'y a pas d'équipe de référence.

**`Option` et non un booléen ni une chaîne vide** : « pas d'équipe de
référence » et « match nul » sont deux choses, qu'un `String::new()`
confondrait.

## Tests

| Test | Ce qu'il prouve |
|---|---|
| `l_onglet_competition_ne_rend_ni_pastille_ni_journee` | les deux `None` |
| `le_composant_rend_le_score_et_les_blessures` | le bloc extrait est complet |
| `le_composant_rend_le_badge_en_cours_de_saisie` | le second état |
| `le_composant_omet_le_lien_sans_url_de_rapport` | le troisième `Option` |

Et le contrôle visuel : `uv run python visual/debordements.py` muet, plus une
comparaison de rendu de la page compétition avant/après.

## Checklist

- [x] `match-widget.html` extrait par **copier-coller**
- [x] `competition-tab-resultats.html` l'appelle, tout le reste inchangé
- [x] `components/match-widget.css`, **inscrite au bundle** (axe 14)
- [x] Les 86 règles retirées de `pages/competition-detail.css`
- [x] `round_label` et `outcome` sur le VM, `None` partout côté compétition
- [x] `debordements.py` — ne signale rien de neuf (7 débordements préexistants)
- [x] La page compétition rend à l'identique — **mesuré**, pas supposé
- [x] `make lint && make test && make check-arch`

---

# Ce que la réalisation a appris

## La feuille ne pouvait pas être dé-préfixée

La carte affirmait : *« Les feuilles de `components/` sont globales par
construction pour ce contrôle : le débordement disparaît de lui-même. »*
**C'est faux**, et vérifié tel quel.

`tests/e2e/visual/debordements.py` n'exempte que `common.css` et
`layout-app.css`, plus **les feuilles scopées par leur propre nom**. Les autres
composants ne passent que parce qu'ils figurent au relevé de référence
`ctrl.json.gz` — la liste de ce que chaque page chargeait avant la fusion des
feuilles. Un fichier neuf n'y a aucune entrée : ses sélecteurs nus auraient été
testés sur les 43 pages du relevé, et `.match-side` comme `.match-team-name`
trouvent du markup sur la page compétition.

Seconde raison, indépendante : **`pages/app-news-feed.css` définit déjà
`.match-team-name`**, avec une autre graisse. Deux feuilles l'auraient posée.

La feuille est donc scopée sous `.match-widget`, du nom du fichier. Trois
bénéfices d'un coup : le contrôle saute la feuille au lieu de la tester, le nom
redevient la portée comme partout ailleurs, et la collision disparaît.

La forme `.match-widget.match-widget--in-progress` n'est pas une coquetterie :
`porte_le_scope` refuse `.match-widget--in-progress`, dont le caractère suivant
la portée est un tiret. La racine porte de toute façon les deux classes.

Et le **double** tiret est un modifieur ; un tiret simple ouvre un autre nom de
classe. `.match-widget-link` est l'ancre de recouvrement, pas une variante du
bloc — une première transposition en avait fait
`.match-widget.match-widget-link`, qui ne sélectionnait plus rien.

## Le composant dépendait encore de la feuille de page

`@keyframes match-pulse`, qui anime le badge « en cours de saisie », était resté
dans `pages/competition-detail.css`. **Le bundle étant unique, l'animation
marchait quand même** — et un nom d'animation est global, donc rien ne l'aurait
signalé. Sur une page qui ne charge pas la feuille de compétition, le badge
aurait cessé de battre sans erreur.

Trouvé en cherchant ce que le composant référence encore : il ne reste que des
tokens de `common.css`.

## « À l'identique » a été mesuré

114 éléments du bloc, **507 propriétés calculées chacun**, plus leur rectangle
englobant, relevés avant et après dans un vrai navigateur : **zéro écart**.

Les deux seuls écarts observés portaient sur l'`opacity` du badge animé — et
varient aussi entre deux relevés du **même** code. C'est l'animation, pas le
déplacement.

## `debordements.py` n'est pas muet, et ne l'était pas avant

Il signale **7 feuilles préexistantes** — `vendor/tom-select.min.css`,
`pages/index.css`, les deux `app-match-report-*`, les deux `app-league-*`,
`app-roster-list` — pour 163 correspondances. Aucune n'est touchée par cette
carte, et aucune ne contient de classe de match. La checklist demandait « muet » ;
c'est un état que le dépôt n'a pas.

Il demande aussi à être lancé **depuis la racine** et non depuis `tests/e2e`
comme sa docstring l'indique : le parseur qu'il emprunte à
`check-css-collisions.sh` lit des chemins relatifs.

## Falsification

| Mutation du gabarit | Constaté |
|---|---|
| La pastille se rend même sans issue | 3 rouges |
| La journée se rend même sans libellé | `l_onglet_competition_ne_rend_ni_pastille_ni_journee` rouge |
| Victoire et défaite interverties | 2 rouges |
| Les blessures disparaissent | `le_composant_rend_le_score_et_les_blessures` rouge |
| Le repli sur les initiales est perdu | idem |
| Le badge « en cours » disparaît | `le_composant_rend_le_badge…` rouge |
| Le lien se rend toujours | `le_composant_omet_le_lien…` rouge |
