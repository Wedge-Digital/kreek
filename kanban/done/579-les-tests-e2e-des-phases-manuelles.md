# Les tests e2e des phases manuelles — sorties et refus

**Priorité : moyenne**
**Épic :** aucune — fonction « phases manuelles », cartes 575 à 579
**Dépend de :** 578
**Spec :** `07-integration.md` (point 8)
**Fichiers :** `tests/e2e/test_manual_phase_override.py` (créé par la 578)

## L'objectif

Prouver dans un navigateur que la fermeture d'une phase manuelle ramène à
« prête à jouer », et que les gardes tiennent au-delà de l'écran.

L'ouverture des trois phases et le refus fait à un simple membre sont écrits
avec la carte 578 (décision du 2026-10-03).

## Les scénarios

1. Le coach dépense des SPP pendant une phase de dépense ouverte à la main, la
   valide ; l'équipe revient prête à jouer sans passer par le recrutement, et sa
   valeur d'équipe intègre l'achat.
2. Une sortie de renvois manuelle au-dessus du seuil ramène à prête à jouer, sans
   écran d'erreurs coûteuses.
3. Une équipe qui n'est plus prête à jouer reçoit le message au pied du panneau.
4. Pendant une phase manuelle, le dernier rapport de match n'est pas
   corrigeable.

## Terminé quand

Les quatre scénarios passent, et la suite e2e complète aussi.
