# Le test de déplacement de match échoue une fois sur sept

**Priorité : moyenne**
**Épic :** aucune
**À programmer :** après les cartes 576 à 579 (décision du 2026-10-03)
**Fichier :** `tests/e2e/test_deplacer_un_match.py`

## Le constat

`test_un_administrateur_deplace_un_match_publie` a échoué dans la suite complète
du 2026-10-03 et passé seul juste après. Il attend en vain une option dans le
sélecteur de journée (`kreek-select[name='round_id']`, ligne 106) : la liste est
vide.

## La cause — le tirage, pas le produit

- La compétition du test a 8 équipes et 7 journées ; le générateur évite les
  rencontres déjà jouées (`generate_pairings.rs`, `already_played`) :
  `teams[0]` et `teams[1]` se rencontrent sur **une seule** journée, tirée au
  hasard.
- Le test publie `teams[0]` contre `teams[1]` sur `round_ids[0]`, puis veut le
  déplacer sur `round_ids[1]`, après `liberer_les_equipes` sur cette journée.
- `liberer_les_equipes` **ne fait rien** quand le couple y est déjà apparié
  (`if deja: return`) — elle suppose que c'est le match que le test veut ouvrir.
- Si le tirage a mis leur rencontre sur `round_ids[1]`, elle y reste ;
  `cibles()` (`move_pairing_widget.rs`) écarte cette journée, et les deux équipes
  sont prises sur toutes les autres. Aucune cible : liste vide.

Environ une chance sur sept par exécution. Le serveur n'avait pas redémarré
(PID inchangé sur toute la suite).

## Le changement

Choisir pour cible une journée, autre que `round_ids[0]`, où `teams[0]` et
`teams[1]` ne sont **pas** déjà face à face — lue en base avant
`liberer_les_equipes`. Le produit ne change pas.

Vérifier au passage si le scénario du refus (« une des deux équipes joue déjà »)
dépend du même hasard.

## Terminé quand

Le test passe sur un tirage où le couple est apparié sur `round_ids[1]` — forcé
ou constaté —, et la suite e2e complète passe.
