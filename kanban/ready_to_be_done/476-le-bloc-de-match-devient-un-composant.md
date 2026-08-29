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

- [ ] `match-widget.html` extrait par **copier-coller**
- [ ] `competition-tab-resultats.html` l'inclut, tout le reste inchangé
- [ ] `components/match-widget.css`, **inscrite au bundle** (axe 14)
- [ ] Les 93 règles retirées de `pages/competition-detail.css`
- [ ] `round_label` et `outcome` sur le VM, `None` partout côté compétition
- [ ] `debordements.py` muet
- [ ] La page compétition rend à l'identique — vérifié, pas supposé
- [ ] `make lint && make test && make check-arch`
