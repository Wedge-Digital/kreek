# K3 — Grilling

> Reconstitué à la migration (2026-11-19), à partir de l'épic `kanban/epics/done/E03-front-ni-saut-ni-clignotement.md` (devenue `initiative.md`). Le dépôt ne garde
> pas de trace d'un grilling : les besoins ci-dessous reprennent l'objectif et les cartes, à relire.

## Questions et réponses

Aucune trace dans le dépôt.

## Besoins

- **K3-B1** — Terminé quand (critère de l'épic E03) : Sur la démo, **aucun contenu n'est peint sans ses styles** au cours d'une navigation HTMX : le document ne se voit insérer aucun `<link>`, et aucune requête ne part vers une feuille qui le style. Et un chargement complet de l'accueil produit un déplacement vertical cumulé de **0 px**, mesuré en desktop et sous 768 px. **Constaté le 2026-08-22**, sur les 11 entrées de navigation du menu : le `<head>` ne porte qu'une feuille — le bundle —, 0 `<link>` inséré, 0 requête CSS. Accueil : 0 px de déplacement, aux deux largeurs. Le critère disait d'abord « ne déclenche **aucune** requête CSS », sans réserve. La carte 17 a montré que cette formulation demandait autre chose que ce que l'épic combat : le widget Cloudinary télécharge une feuille **pour son propre iframe**, qui ne style aucun de nos éléments et ne peut donc rien faire clignoter. La version ci-dessus nomme le phénomène au lieu de son indice. Ce n'est pas un abaissement du critère, et on peut le vérifier : les deux pages qui portent un champ d'upload ont le même compte de `<link>` avec ou sans ce téléchargement — 1 dans les deux cas, mesuré. Ce qu'on retire de la formulation n'avait aucun effet sur ce qu'elle voulait garantir. La leçon vaut au-delà : **un critère observable peut être observable et faux** s'il mesure un indice plutôt que la chose.
- **K3-B2** — Dépendances CDN en production (ticket `17-dependances-cdn`)
- **K3-B3** — CSS — donner sa portée à chaque feuille, à rendu constant (ticket `341-css-solder-les-collisions-de-selecteurs`)
- **K3-B4** — CSS — les feuilles réunies en un fichier unique chargé dans le `<head>` (ticket `342-css-fusion-en-fichier-unique`)
- **K3-B5** — Chrome et widgets différés — réserver la place avant le remplissage (ticket `343-reserver-la-place-des-zones-differees`)

## Hors périmètre

Voir la section « Ce que l'épic ne couvre pas » de `initiative.md`, s'il y en a une.
