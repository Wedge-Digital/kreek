# Le recrutement clique avant le câblage

**Priorité : haute** — la suite e2e doit passer à chaque fois (décision du 2026-11-11)
**Épic :** aucune · **Dépend de :** 662 (le patron du correctif)
**Trouvée par :** les essais de la 662 (2026-11-11).

## Le constat

`test_recruitment_phase.py` a le défaut de la 662 : `_acheter` (ligne 143) clique
sans attendre le câblage et ne guette que le premier signe visible. Un achat perdu
fait tomber ensuite les scénarios 04, puis 08 et 10, qui partagent l'équipe : 3
tests sur 22, environ une fois sur dix.

## Ce qui change (tests seulement)

Le patron de la 662 : mutations par une aide qui attend le repos, clique quand le
bouton est câblé, attend le nombre de rendus attendus puis le repos ; la réponse
de la mutation encadrée par `expect_response`, pour un message d'échec explicite ;
le décor de chaque scénario construit par lui-même, sans dépendre du panier laissé
par le précédent.

## Terminé quand

- [ ] le fichier passe **20 fois de suite**, seul puis précédé d'un fichier lourd ;
- [ ] la suite complète passe.

## Conséquences sur les données de production

Aucune : seuls des tests changent.
