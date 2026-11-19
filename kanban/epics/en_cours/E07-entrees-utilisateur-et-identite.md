# E07 — Entrées utilisateur et identité

**État :** `en_cours` — 2 cartes, **1 faite** : la 329 est livrée par le
commit `19c1f76`, qui a sorti les noms de la liste blanche. Reste la 328.

## La fonction

Ce que l'application accepte d'un utilisateur peu fiable — non connecté, ou
simplement en train de taper un nom — et ce qu'elle en fait ensuite. Les deux
cartes vivent dans `auth` et `shared_kernel::identity`, et corrigent le même
travers par les deux bouts : une règle trop laxiste d'un côté, trop stricte de
l'autre.

## Les cartes

| # | Intitulé | Apport |
|---|---|---|
| 328 | Un lien partagé survit à la page de connexion | `?next=` reconstruit, jamais filtré |
| 329 | Les noms sortent de la liste blanche | une liste noire commune à cinq value objects |

## Ce qui commande l'ordre

Aucune dépendance entre les deux. Elles se font dans n'importe quel ordre, ou
en parallèle.

**328 est la plus délicate**, et son risque est nommé dans la carte : sans
garde, `?next=` transforme la page de connexion en tremplin de hameçonnage — le
lien porte notre domaine, affiche notre formulaire, et dépose la victime sur un
site tiers une fois le mot de passe saisi. La carte impose la seule approche qui
tienne : **reconstruire plutôt que filtrer**, valider la valeur *décodée*, et
valider *à l'émission* et non seulement à la capture. Le tableau des huit
valeurs à faire échouer est le test unitaire de la carte.

**329 est la plus large** — elle touche `SpaceName`, les deux `TeamName`,
`CoachName`, `SeasonName` et `TierName` — mais son risque est faible : on
élargit, donc aucun nom existant ne devient invalide, et il n'y a rien à
migrer en base.

Un point de la 329 mérite l'attention au démarrage : `CoachName` **est
l'identifiant de connexion**, avec une unicité octet par octet
(`users_coach_name_uq`). Il reçoit donc une restriction en plus — refus des
`\p{Cf}` — pour fermer l'usurpation par caractère invisible.

## Ce que l'épic ne couvre pas

- **L'autorisation par rôle.** Qui a le droit de faire quoi une fois connecté
  est un sujet distinct, et il n'a pas encore sa carte.
- **Le cloisonnement des espaces** — c'est E04 (`316`, `323`).
- **Les homoglyphes inter-alphabets** (`а` cyrillique contre `a` latin) sur le
  nom de coach. La 329 ferme le cas des caractères invisibles, pas celui-là :
  il demande une normalisation et une table de confusables, donc une carte à
  part si le besoin se présente.
- **L'inscription et la réinitialisation de mot de passe**, que la 328 laisse
  hors périmètre : un lien partagé mène à une connexion, couvrir ces deux
  parcours doublerait la surface de test pour un cas rare.
- `432-rendre-accessible-un-espace-publiquement`. Sujet voisin — qui a le droit
  de lire quoi sans être connecté — mais sa **portée n'est pas tranchée** :
  lecture pour tout visiteur, pour tout coach connecté, ou vitrine choisie page
  par page. La première touche `space_scope`, le verrou de la carte 416, et
  demande un chemin distinct plutôt qu'un assouplissement. Rien à rattacher tant
  que ce choix n'est pas fait.

## Terminé quand

Un lien profond envoyé à quelqu'un de non connecté le mène à la page demandée
après sa connexion, chaîne de requête intacte ; `?next=https://evil.example/`
le mène à l'accueil ; et `L'Ost & Cie` est un nom d'équipe accepté.
