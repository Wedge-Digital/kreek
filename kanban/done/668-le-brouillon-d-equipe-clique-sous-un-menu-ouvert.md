# Le brouillon d'équipe clique sous un menu ouvert

**Priorité : haute** — la suite e2e doit passer à chaque fois (décision du 2026-11-11)
**Épic :** aucune · **Dépend de :** rien
**Trouvée par :** la suite complète de stabilisation (2026-11-11).

## Le constat

`test_draft_team_errors.py::test_submit_without_coach_shows_error` échoue 2 fois
sur 5, seul : le clic sur « Créer l'équipe → » attend 30 s, parce qu'une option
du `kreek-select` de la compétition (`.ks-option`) recouvre le bouton — le menu
est resté ouvert après le choix.

**Non établi** : le menu reste-t-il ouvert pour un vrai utilisateur (défaut de
`kreek-select.js`, par exemple un rechargement des options qui le rouvre), ou
seulement parce que le test clique pendant un rendu ? Le fichier attend en outre
par quatre pauses fixes (`wait_for_timeout`, lignes 26, 51, 65 et 78).

## Ce qui change

1. Établir la cause par une mesure, pas par une supposition.
2. Si le menu reste ouvert pour un utilisateur : corriger `kreek-select.js`, avec
   un test qui échoue sans le correctif.
3. Côté test : attendre la fermeture du menu après le choix, et remplacer les
   quatre pauses par des conditions (règle « pas de `sleep` » de `CLAUDE.md`).

## Terminé quand

- [ ] le fichier passe **20 fois de suite** ;
- [ ] la suite complète passe.

## Conséquences sur les données de production

Aucune attendue ; à confirmer si `kreek-select.js` change (composant partagé par
tous les sélecteurs).
