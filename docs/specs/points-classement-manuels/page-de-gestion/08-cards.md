# Points de classement manuels · Phase 8 : cartes kanban

**Phases 1 à 7** : ce dossier

Cinq cartes, en `kanban/ready_to_be_done/`.

| N° | Carte | Dépend de |
|---|---|---|
| 449 | Le domaine sait porter un point manuel | rien |
| 450 | La table des points manuels | rien |
| 451 | Le classement affiche les points manuels | 449, 450, **et la carte 448** |
| 452 | La page de gestion | 450, 451 |
| 453 | Les tests E2E | 452 |

**449 et 450 sont indépendantes** et se prennent dans n'importe quel ordre.

## Quatre choix de découpage

**449 est seule parce qu'elle touche `compare`** — la fonction qui ordonne
**tous** les classements de l'application. Une régression là-dessus se voit sur
chaque compétition, et elle doit se relire sans être mêlée à une table neuve.
Ses onze tests sont son intérêt : ils prouvent la règle avant qu'aucun écran
n'existe.

**451 déclare la carte 448 en prérequis.** Elle ajoute une colonne à un tableau
dont le survol est invisible une ligne sur deux — même fichier, même geste. La
448 reste autonome et vaut pour tout le site ; elle passe simplement avant, ou
avec.

**452 vient après 451, à rebours de l'ordre naturel.** On écrirait d'abord la
saisie. Mais sans l'affichage, la page de gestion livre un écran dont on ne peut
vérifier aucun effet : on attribue trois points et rien ne bouge nulle part.

**Les tests E2E en une carte** : les scénarios traversent les deux écrans —
attribuer d'un côté, vérifier le rang de l'autre. Répartis, ils seraient écrits
deux fois à moitié.

## Ce que l'ensemble n'emporte pas

- **Aucune modification d'une ligne** : elle se supprime.
- **Aucun événement, aucun listener, aucun rejeu** — le classement s'ordonne à
  chaque lecture.
- **Aucune migration de données** : la table naît vide.

## Ce que la phase 8 clôt

Le workflow s'arrête ici. L'implémentation se fait carte par carte, sous les
règles ordinaires du `CLAUDE.md`.
