# E08 — Mode customisation : finir la livraison

**État :** 2 cartes · 0 faite

## La fonction

Le mode de customisation des joueurs (cartes 302 à 309) est livré et
utilisable, mais deux défauts le rendent pénible à l'usage réel :

- l'onglet actif — « Caractéristiques », « Prix », « SPP » — **se perd à chaque
  enregistrement** et retombe sur « Compétences » ;
- la somme de **SPP disponibles ne suit pas** : l'utilisateur voit l'ancienne
  valeur alors que la modification est bien enregistrée.

Sur une session réelle — plusieurs lignes posées, validées, ajustées — la
manipulation se répète à chaque tour, et le chiffre affiché ne peut pas servir
de repère. L'épic rend le mode utilisable en continu.

## Les cartes

| # | Intitulé | Apport |
|---|---|---|
| 326 | L'onglet actif ne survit pas à l'enregistrement | l'onglet est restauré après le `HX-Refresh` |
| 327 | La somme de SPP disponibles ne suit pas | les quatre affichages de la réserve concordent |

Les deux sont en **priorité haute**, et ce sont les seules cartes du backlog
qualifiées de « bug de livraison ».

## Ce qui commande l'ordre

**Commencer par la question de diagnostic de la 327**, avant de coder quoi que
ce soit : reproduire, puis recharger la page à la main.

- La valeur devient correcte au F5 → c'est un problème de rafraîchissement de
  fragment, et **les deux cartes tiennent probablement la même racine** : elles
  se referment d'un coup.
- La valeur reste fausse → c'est le calcul lu, et les deux cartes divergent. Il
  faut alors départager `compute_spp_breakdown`
  (`player_detail_controller.rs:353`) et `reserve_effective`
  (`player_customisation_widget.rs:349`) — deux calculs de « réserve » qui
  coexistent.

Trancher cette question avant de proposer un correctif : les deux branches ne
mènent pas au même fichier.

Ce qui n'est **pas** à re-vérifier, la 327 l'a déjà fait : le chemin d'écriture
est sain. `Player::apply` sur `PlayerSppCustomised` ajoute bien le montant, et
la projection fait `UPDATE players_proj SET spp = spp + $2` dans la transaction
de l'append.

Pour la 326, la carte recommande explicitement la seconde des deux voies :
faire **survivre l'onglet** au rechargement (fragment d'URL) plutôt que
remplacer le `HX-Refresh` par des swaps ciblés. Le rafraîchissement complet
garde sa garantie de cohérence — la validation change réellement les quatre
zones de la fiche — et toute zone oubliée par un swap ciblé deviendrait périmée
en silence.

## Ce que l'épic ne couvre pas

Le mode **« dépense de SPP »** — `purchase_skill_controller.rs:83` et
`increase_stat_controller.rs:67` renvoient eux aussi un `HX-Refresh`, mais leur
panneau n'a pas d'onglets. À regarder pendant la correction ; à traiter dans une
carte à part seulement si le même symptôme s'y reproduit.

## Terminé quand

Un coach enchaîne trois customisations depuis l'onglet « SPP » sans jamais
reprendre la souris pour revenir à cet onglet, et le chiffre de réserve affiché
après la troisième est le bon — panier en attente compris.
