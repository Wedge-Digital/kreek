# Le récapitulatif annonce un faux blocage juste après la publication

**Priorité : à décider**
**Épic :** aucune · **Dépend de :** rien
**Trouvée par :** le diagnostic de la carte 663 (2026-11-11).

## Le constat

Deux défauts produit, vus en diagnostiquant un test :

1. **Un faux motif.** Juste après la publication, le coach redirigé sur le
   récapitulatif peut voir le bouton de correction désactivé, avec le motif « une
   équipe a quitté la phase d'amélioration » : le listener `teams` n'a pas encore
   fait entrer les équipes dans cette phase (`correction_eligibility_service`,
   `is_in_post_match_improvement`). Le motif est faux ; il redevient juste quelques
   dizaines de millisecondes plus tard.
2. **Des gains perdus sans bruit.** `create_draft` accepte une équipe en pleine
   séquence post-match ; `teams` refuse ensuite la seconde séquence (« phase de jeu
   incorrecte ») et les gains du second match sont perdus, sans message au coach.

## Questions ouvertes

- Le récapitulatif doit-il distinguer « en cours d'entrée en phase d'amélioration »
  d'« a quitté la phase », ou attendre l'état de `teams` (direct E23) ?
- Le démarrage d'un match doit-il refuser une équipe en séquence post-match, et par
  quelle consultation (port vers `teams`) ?

## Conséquences sur les données de production

À établir : rechercher en production les séquences post-match refusées pour
« phase de jeu incorrecte », qui sont autant de gains perdus.
