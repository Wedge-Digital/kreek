# Démarrer depuis le calendrier sans sélection

**Épic :** E21 — Le mode de saisie et l'après-match en séquence · **Ordre :** 3 · **Dépend de :** 632
**Conception :** `docs/specs/saisie-mobile/E21-cadrage.md` (R3, R4, § 4)
**Maquette :** `assets/rawpages/html/app-mobile-match-mode.html`

## Objectif

Depuis le calendrier, « Commencer » mène à la question « Qui saisit ce
match ? » puis droit à l'avant-match : plus d'écran de sélection des équipes
pour un match qui est déjà au calendrier. « Nouveau match » hors calendrier
garde la sélection (`/new`).

## Conception

- `from_pairing` ne change pas : il redirige vers `/{id}`.
- `edit_match_report`, branche `Draft` (`io/web/match_selection_controller.rs:244-273`) :
  rend un gabarit `match-mode.html` — le match (compétition, journée, deux
  équipes, résolus par les ports existants comme `selection_figee`), le
  fragment de 632, « Commencer → ». **Aucune confirmation au GET** : ouvrir le
  lien ne verrouille rien (`MatchReporting`) et ne passe pas le match « en
  cours ».
- `update_match_selection` (POST `/{id}`) confirme **les équipes du brouillon**
  avec le mode ; celles du formulaire sont ignorées, pour tous — admin compris.
  `equipes_retenues` et `selection_figee` n'ont plus de cas « sélection
  modifiable » sur un brouillon du calendrier : leur sort suit la règle 4 de
  `CLAUDE.md` (consommateurs listés avant suppression).
- La confirmation émet toujours `SelectionConfirmed`, donc
  `MatchReportConfirmed` par le publisher (352).
- Deux « Commencer » simultanés : le second reçoit 409 et la page se recharge
  (623, 631) ; il arrive sur l'état du match.

**Un brouillon d'origine `Manual`** ne devrait pas exister (confirmé à la
création, `create_match_report_use_case.rs:82-84`) ; s'il en reste d'avant la
carte 552, il garde la sélection. À vérifier sur un dump.

## Données de production

Aucune. Chaque brouillon du calendrier devient démarrable sans sélection, par
le seul bouton. **Perte acceptée** (2026-11-01) : on ne change plus les équipes
ni la journée d'un match du calendrier par la sélection — le déplacement admin
reste.

## Checklist

- [ ] `match-mode.html` rendu pour un brouillon du calendrier
- [ ] POST `/{id}` : équipes du brouillon, mode du formulaire
- [ ] Ce qui ne sert plus dans le contrôleur de sélection : consommateurs listés, puis retiré
- [ ] Tests unitaires : le GET ne confirme rien ; le POST ignore des équipes envoyées
- [ ] E2E : `test_competition_matchs.py` C7 migré — la ligne du calendrier mène à « Qui saisit ? », « Commencer » mène à l'avant-match
- [ ] Mesure sur dump : brouillons `Manual` restants
- [ ] `tests/impact-map.toml` mis à jour
- [ ] `make lint`, `make check-arch`, `make test`, e2e impactés
