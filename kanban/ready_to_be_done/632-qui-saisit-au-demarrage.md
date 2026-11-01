# « Qui saisit ? » au démarrage

**Épic :** E21 — Le mode de saisie et l'après-match en séquence · **Ordre :** 2 · **Dépend de :** 631
**Conception :** `docs/specs/saisie-mobile/E21-cadrage.md` (R1, § 4, § 8.1, § 8.2)
**Maquette :** `assets/rawpages/html/app-mobile-match-mode.html`

## Objectif

Celui qui démarre un match choisit qui saisit. Une carte « Qui saisit ce
match ? » apparaît sur l'écran de sélection, ordinateur et mobile, cochée sur
« 📱 Je saisis tout ». Le choix est dit définitif à l'écran : « Le mode vaut
pour tout le match : il ne pourra plus changer. »

## Conception

- Un fragment Askama commun, inclus par `match-selection.html` : deux options,
  « Je saisis tout » cochée, « 👥 Chacun son équipe » grisée et marquée
  « bientôt » (décision du 2026-11-01). Un champ
  `entry_mode` dans le formulaire.
- `CreateMatchReportForm` et `UpdateMatchSelectionForm` lisent `entry_mode`.
  Le contrôleur le traduit en `EntryMode` ; une valeur inconnue répond 400.
  Absent : `SoloEntry`, le défaut déclaré (décision du 2026-11-01).
- `CreateMatchReportCommand`, `confirmer_si_brouillon` et
  `UpdateMatchSelectionCommand` portent le mode jusqu'à `confirm_selection`
  (631).
- Mêmes routes, même gabarit sur les deux appareils : la carte s'adapte sous
  768 px dans sa propre feuille, inscrite au bundle.

**Ce que la carte ne fait pas** : retirer la sélection des matchs du calendrier
(633), prévenir l'adversaire (E23).

## Données de production

Aucune.

## Checklist

- [ ] Le fragment « Qui saisit ? » et sa feuille, dans `css_bundle.rs`
- [ ] `entry_mode` lu par les deux formulaires et porté par les trois commandes
- [ ] Tests unitaires : traduction du champ (valeur connue, inconnue, absente)
- [ ] E2E : la carte est visible et cochée sur `/new` ; un match créé en sort en « Je saisis tout » (`match_report_proj.entry_mode`)
- [ ] `tests/impact-map.toml` mis à jour
- [ ] `make lint`, `make check-arch`, `make test`, e2e impactés
