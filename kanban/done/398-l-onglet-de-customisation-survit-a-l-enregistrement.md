# `players` — L'onglet de customisation survit à l'enregistrement

**Priorité : haute** — bug de livraison du mode de customisation
**Fusionne :** les cartes **326** et **327**, toutes deux en `cancelled/`
**Dépend de :** rien
**Fichiers :** `src/app/players/io/web/templates/player-customisation-widget.html`,
et un test e2e

## Pourquoi une seule carte

Les deux cartes visaient le même écran, et l'on croyait qu'elles partageaient
une cause. **C'est pire que ça : elles étaient en tension.** Ce qui empêche le
défaut de la 327 est exactement ce que la 326 proposait de supprimer.

Établi par la mesure, sur un joueur à 16 SPP, en ajoutant 5 :

| Moment | En-tête | Panneau | Résumé | Base |
|---|---|---|---|---|
| Départ | 16 | 16 | 16 | **16** |
| Après « Ajouter » | 16 | **21** | 16 | **16** |
| Après « Enregistrer » | 21 | 21 | 21 | **21** |

**La 327 ne se reproduit pas.** Après enregistrement, les quatre affichages
sont justes et cohérents avec la base, sans double comptage. La raison tient en
une ligne : `customisation_controller.rs:378` renvoie `HX-Refresh: true`, la
page entière se recharge, et les quatre zones repartent du serveur.

La divergence observée n'existe qu'**avant** l'enregistrement. « Ajouter » ne
sauve rien — la ligne va au panier, la base reste à 16. Le panneau affiche
alors la réserve *projetée*, panier compris ; l'en-tête, la réserve *réelle*.
Les deux calculs que la 327 soupçonnait — `compute_spp_breakdown` et
`reserve_effective` — divergent bel et bien, et c'est **délibéré**.

Ce qu'aucun des deux ne dit à l'écran, c'est qu'ils ne mesurent pas la même
chose. C'est très probablement ce qui a été pris pour un défaut de mise à jour.

**Conséquence pour la correction :** remplacer le rechargement par des swaps
ciblés — l'option 1 de la 326 — ferait apparaître pour de bon le défaut que la
327 décrivait. Cette voie est écartée, et il ne faut pas la rouvrir sans
relire ce qui précède.

## Le défaut qui reste, et lui seul

Chaque enregistrement recharge la page. L'utilisateur qui travaillait dans
« Caractéristiques », « Prix » ou « SPP » se retrouve sur « Compétences », et
doit re-cliquer pour continuer. Sur une session de customisation — plusieurs
lignes posées, validées, ajustées — la manipulation se répète à chaque tour.
Reproduit.

L'onglet actif ne vit que dans l'état Alpine local du panneau
(`player-customisation-widget.html:15`) :

```html
x-data="{ activeTab: 'skills', skillSearch: '' }"
```

Un rechargement complet réinitialise cet état. Le commentaire du contrôleur dit
vrai — la validation change bien les quatre zones de la fiche — donc ce n'est
pas le rafraîchissement qui est fautif, c'est l'onglet qui n'a nulle part où
survivre.

## Ce qu'il faut faire

**Faire survivre l'onglet au rechargement.** Persister `activeTab` dans le
fragment d'URL et le relire à l'`init()` du composant Alpine. Le rafraîchissement
complet reste, avec sa garantie de cohérence ; seul l'onglet est restauré.

Un fragment (`#custo-spp`) plutôt qu'un `sessionStorage` : il est visible,
partageable, et disparaît de lui-même quand on quitte la fiche — pas de résidu
à nettoyer, ni de clé à indexer par joueur.

## Portée — ce que cette carte ne traite pas

`purchase_skill_controller.rs:83` et `increase_stat_controller.rs:67` renvoient
eux aussi un `HX-Refresh`. Ils appartiennent au mode « dépense de SPP », pas au
mode customisation, et leur panneau n'a pas d'onglets. **Les regarder pendant
la correction**, mais ne les modifier que si le même symptôme s'y reproduit —
sinon, une carte à part.

**L'ambiguïté des deux réserves** — projetée contre réelle — n'est pas traitée
ici. Elle est réelle, elle a coûté une carte, et elle mérite la sienne : soit
un libellé qui distingue les deux, soit un signe que le panier n'est pas vide.

## Checklist

- [x] L'onglet actif est restauré après l'enregistrement d'une customisation
- [x] Il l'est aussi après une **annulation de ligne**
- [x] Rien ne subsiste quand on revient sur la fiche par une autre voie —
      l'onglet par défaut reste « Compétences » à l'arrivée
- [x] Test e2e vu échouer avant correction, sur les deux passages
- [x] `make test` passe — 1237 tests
- [x] `make check-arch` passe

### Le vidage du panier n'avait pas d'objet

La case demandait que l'onglet survive aussi au vidage du panier. **Il n'y a
rien à restaurer** : `btn-cancel-all` ferme le mode customisation, ce qui est
voulu et déjà tenu par `test_annuler_vide_le_panier_et_ne_rouvre_pas_le_mode`.
Écrit ici plutôt que coché.

## Ce qui a été fait

Trois lignes dans le `x-data` du panneau : la liste des onglets connus, la
lecture du fragment à l'`init()`, et un `$watch` qui l'écrit à chaque bascule.

`history.replaceState` et non `location.hash` : ce dernier empile une entrée
d'historique par clic, et le bouton Retour se mettrait à défiler les onglets au
lieu de quitter la page.

Rien n'est écrit tant qu'aucun onglet n'a été choisi — c'est ce qui garde
« Compétences » à l'arrivée, et l'URL propre.

### Le test a d'abord passé sans rien vérifier

Sa première version lisait l'onglet **immédiatement** après le clic d'ajout.
Le remplacement du panneau n'était pas encore arrivé, elle lisait donc l'ancien
DOM — où « SPP » est toujours actif — et passait sans le correctif. Corrigée en
attendant d'abord la disparition de « Aucune modification en attente », elle
échoue comme il faut : `unexpected value "Compétences"`.

Deuxième piège du même ordre : `.custo-action-panel:visible` désigne le mauvais
panneau juste après un remplacement, quand Alpine n'a pas encore appliqué
`x-show` et que les quatre sont visibles. Les sélecteurs repèrent donc le
panneau **par son contenu** — le champ `amount` n'existe que dans celui des
SPP.
