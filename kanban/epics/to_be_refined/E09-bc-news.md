# E09 — BC `news`

**État :** 2 cartes · 0 faite · **contenu à définir**

## La fonction

Le BC `news` existe et sert déjà un fil d'articles (`news-feed.html`,
`app-article-detail`, `app-create-article`). Ce qui manque, c'est ce qui en
fait un outil de communication de ligue plutôt qu'une liste : **soumettre** un
article, et **épingler** celui qui doit rester en tête.

## Les cartes

| # | Intitulé | État |
|---|---|---|
| 50-NEWS-BC-submit-article | Soumission d'un article | **« contenu à définir »** |
| 50-NEWS-BC-pin-article | Épinglage d'un article | **« contenu à définir »** |

Les deux fiches sont des marque-pages : elles portent un titre et la mention
« contenu à définir », rien d'autre.

## Ce qui commande l'ordre

Rien n'est décidable avant le raffinage, et le raffinage bute sur des questions
qui n'ont pas de réponse dans le dépôt :

- **Qui soumet ?** N'importe quel membre de l'espace, ou les seuls admins ? Si
  c'est ouvert, faut-il une modération — et donc un état « en attente » ?
- **L'épinglage est-il un attribut de l'article ou une décision d'espace ?**
  Combien d'articles épinglés simultanément ?
- **Un article appartient-il à un espace, à une compétition, ou aux deux ?**
  La question décide du cloisonnement, donc du résolveur
  `ISpaceOwnership` à écrire.

Ces trois questions sont le vrai travail de l'épic. Tant qu'elles sont
ouvertes, il n'y a pas de plan à faire.

## Ce que l'épic ne couvre pas

Le fil d'articles existant et sa page de détail, qui fonctionnent.

## Terminé quand

Un membre de l'espace publie un article depuis l'application, et un
administrateur peut le faire remonter en tête du fil.
