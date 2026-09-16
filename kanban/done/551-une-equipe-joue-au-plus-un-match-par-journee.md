# Une équipe joue au plus un match par journée

**Priorité : haute — préalable à la carte 552**
**Épic :** aucune — une règle, livrable d'un bloc
**Dépend de :** rien
**Fichiers :**
`src/app/competitions/domain/match_day.rs`,
`src/app/competitions/domain/error.rs`,
`src/app/competitions/use_cases/admin/add_match_use_case.rs`,
`src/app/competitions/use_cases/admin/generate_pairings.rs`,
`src/app/competitions/io/web/admin/schedule_widgets.rs`,
`tests/e2e/` *(nouveau test)*, `tests/impact-map.toml`

## L'objectif

Poser l'invariant **« une équipe joue au plus un match par journée »** dans
`MatchDay`, seul point que les trois chemins de création d'appariement
traversent.

## Ce qui l'a fait naître

L'invariant existe déjà — mais comme détail d'algorithme, pas comme règle du
domaine. `generate_round_pairings` maintient un `used: HashSet<&str>` qui
empêche une équipe d'être appariée deux fois pendant un tirage. Il ne protège
que ce chemin-là.

`add_match_use_case`, par lequel un commissaire ajoute une rencontre à la main
au calendrier, ne vérifie **que l'enrôlement** des deux équipes. Ni doublon de
couple, ni équipe déjà occupée ce jour-là. Le trou est donc ouvert côté admin
depuis toujours, indépendamment de l'incident du 15 septembre qui a révélé le
troisième chemin (cf. carte 552).

C'est la forme classique d'une règle qui tient par accident : elle est vraie
partout où on l'a écrite, et fausse partout où on a oublié de l'écrire.

## Le comportement

- **Refus** si l'une des deux équipes porte déjà un appariement sur cette
  journée.
- Le refus **nomme l'adversaire déjà programmé**. Un message qui dit seulement
  « impossible » envoie l'admin fouiller le calendrier pour comprendre ce qui
  bloque — et c'est précisément l'information que le domaine vient de lire.
- Le couple déjà programmé n'est pas un cas à part : si A-B existe, alors A joue
  déjà, et la même règle le refuse. Aucune vérification supplémentaire à écrire.

## Où vit la règle

Dans le domaine, sur `MatchDay`, qui porte déjà ses `pairings`. Pas dans chaque
use case : trois copies divergeraient, et l'écart se verrait sous la forme d'un
chemin qui accepte ce qu'un autre refuse — ce qui est exactement l'état actuel.

`generate_pairings` ne doit pas changer de comportement : son `used: HashSet`
produit déjà des journées conformes. L'invariant devient sa filet, pas son
mécanisme.

## Tests

- **Unitaire domaine** : les trois cas — équipe à domicile déjà programmée,
  équipe à l'extérieur déjà programmée, couple identique. Le cas « extérieur »
  est celui qu'une implémentation ne regardant que `home_team_id` laisserait
  passer, et il représente la moitié des rencontres.
- **E2E** : un commissaire tente d'ajouter une rencontre pour une équipe déjà
  programmée ce jour-là et lit le refus, avec le nom de l'adversaire.
- Contre-épreuve : le tirage automatique d'une journée complète passe toujours.

## Terminé quand

Un commissaire qui ajoute à la main une rencontre pour une équipe déjà
programmée cette journée-là obtient un refus nommant son adversaire, et la
génération automatique d'une journée produit le même calendrier qu'avant.
