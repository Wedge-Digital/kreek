# Les tests e2e des phases manuelles

**Priorité : moyenne**
**Épic :** aucune — fonction « phases manuelles », cartes 575 à 579
**Dépend de :** 578
**Spec :** `07-integration.md` (point 8)
**Fichiers :** `tests/e2e/test_manual_phase_override.py` *(nouveau)*,
`tests/impact-map.toml`

## L'objectif

Prouver dans un navigateur que l'ouverture et la fermeture d'une phase manuelle
fonctionnent, et que les gardes tiennent au-delà de l'écran.

## Les scénarios

1. L'admin ouvre chacune des trois phases depuis le bandeau ; l'équipe passe
   dans la phase, avec son bandeau habituel.
2. Le coach dépense des SPP pendant une phase de dépense ouverte à la main, la
   valide ; l'équipe revient prête à jouer sans passer par le recrutement, et sa
   valeur d'équipe intègre l'achat.
3. Une sortie de renvois manuelle au-dessus du seuil ramène à prête à jouer, sans
   écran d'erreurs coûteuses.
4. Un simple membre ne voit pas le bouton ; sa requête forgée reçoit 403.
5. Une équipe qui n'est plus prête à jouer reçoit le message au pied du panneau.
6. Pendant une phase manuelle, le dernier rapport de match n'est pas
   corrigeable.

## Terminé quand

Les six scénarios passent, et la suite e2e complète aussi.
