# Panneau d'ajustement · Phase 8 : les cartes

**Entrée** : `07-integration.md` validé.

## Les quatre cartes

| # | Intitulé | Ce qu'elle livre | Ce qu'on peut constater à la fin |
|---|---|---|---|
| **584** | Le domaine sait ajuster une caisse | value objects, neuvième motif, événement, méthode d'agrégat | `cargo test` au vert sur les treize règles — aucun écran |
| **585** | L'ajustement s'écrit, et le relevé le raconte | use case, commande, bras de `detail_de()` | une ligne `AdminAdjustment` en base, lisible « Par … — … » |
| **586** | Le panneau de modification de la trésorerie | droit, VMs, route, contrôleur, gabarits, Alpine, CSS | un commissaire ajuste une caisse à l'écran |
| **587** | Les tests e2e de l'ajustement | cinq scénarios Playwright | la garde tient au-delà du JS |

## Ce qui commande l'ordre

**584 d'abord, et seule**, parce qu'elle casse volontairement la compilation à
deux endroits : `to_app_event()`, qui n'a pas de joker et exigera de nommer la
variante, et `garde_d_exhaustivite`, qui ramènera à `ALL`. Ces deux ruptures sont
le mécanisme de sûreté du BC ; les subir au milieu d'une carte qui fait aussi de
l'écran les rendrait pénibles plutôt qu'utiles.

**585 avant 586** parce que le bras de `detail_de()` est ce qui rend la ligne
lisible. Dans l'ordre inverse, la 586 afficherait un relevé où l'ajustement
apparaît sans détail — un faux défaut à diagnostiquer.

**587 en dernier**, mais pas en option : les deux scénarios de refus serveur sont
les seuls à prouver que les garde-fous Alpine ne sont pas la seule défense.

## Ce que le découpage ne fait pas

**Il ne sépare pas le handler du gabarit.** La 586 est la plus grosse des quatre
— droit, deux view models, route, contrôleur, deux gabarits, Alpine et CSS. Les
couper donnerait un bouton sans panneau, ou un panneau sans route : deux états
intermédiaires qu'on ne peut ni constater ni livrer. C'est la convention du
workflow — « une carte par widget : handler + template + route ».

**Il ne traite pas l'encapsulation de `Team`.** Carte **583**, `to_be_refined` —
25 champs publics, 139 lectures, 33 fichiers. La 584 le note sans y toucher.

## Aucune migration

Rappelé ici parce que c'est contre-intuitif : le motif et le nom de l'auteur
voyagent dans l'événement, que `list_treasury_movements.sql` joint déjà à chaque
ligne du grand livre. Aucune colonne, aucune table, aucun `sqlx migrate`.
