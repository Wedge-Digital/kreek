# Les statistiques de l'espace

**Priorité : moyenne**
**Dépend de :** 366 et 368 — parallélisable avec 369
**Conception :** `docs/specs/space-admin/membres/04-dtos.md`
**Fichiers :** `io/web/controllers/widgets/space_admin_stats_widget.rs`,
`io/web/templates/widgets/space-admin-stats.html`,
`assets/static/css/widgets/space-admin-stats.css`

## Objectif

Trois compteurs en haut de l'onglet Membres : membres, administrateurs,
invitations en attente.

## Une seule requête

Membres et administrateurs se comptent sur la liste rendue par
`list_members_with_profile`. Un `SELECT count(*)` séparé donnerait deux lectures
pour une donnée que la première contient déjà.

## Le troisième compteur vaut zéro, et c'est délibéré

Les invitations d'espace **n'existent pas** — ni table, ni use case. Elles
arrivent avec leur onglet, qui n'aura qu'une requête à ajouter ici.

L'alternative — livrer deux compteurs et rouvrir la carte plus tard — découpe un
widget en deux moitiés dont la seconde n'a pas de valeur propre. Le zéro est
honnête : il n'y a effectivement aucune invitation en attente, faute
d'invitations tout court.

## Le rafraîchissement

`hx-trigger="load, memberAdded from:body, memberRemoved from:body,
memberRoleChanged from:body"`.

Le widget se rafraîchira aussi quand l'action de rôle est un repost du rôle
courant — compteurs identiques, requête sans effet. C'est assumé : le prix de
l'éviter est une branche conditionnelle dans le handler et une asymétrie à
documenter, pour économiser un aller-retour.

`memberAdded` vient de l'onglet Ajout direct, qui n'existe pas encore. Le
contrat est posé maintenant pour qu'il n'ait rien à renégocier.

## Checklist

- [x] Route `SPACE_ADMIN_STATS_WIDGET`, garde `is_admin()`
- [x] Une seule lecture, les deux compteurs dérivés de la même liste
- [x] Troisième compteur à zéro, avec son motif et son test
- [x] Les quatre déclencheurs `hx-trigger`, `memberAdded` compris
- [x] Racine en `hx-disinherit="*"`
- [x] Feuille nommée d'après la racine, inscrite dans `FEUILLES_APP`
- [x] Quatre tests de harnais, plus **deux scénarios e2e** — le harnais ne peut
      pas vérifier que les compteurs *réagissent*, seulement ce qu'ils comptent
- [x] `make lint`, `make check-arch`, `make test` passent — 1207 tests

## Ce qu'on a appris en la faisant

**Le changement de rôle n'avait jamais fonctionné dans un navigateur.** Deux
causes cumulées, trouvées en mesurant et non en déduisant.

`kreek-select` **ne dispatchait aucun `change` sur lui-même** — il émet sur
`document.body` un événement optionnel, rien d'autre. Le `hx-trigger="change"`
posé dessus n'est jamais parti. Le composant émet désormais un `change`
bouillonnant, ce que tout contrôle de formulaire doit faire ; un seul
consommateur en dépendait.

Une fois ce point corrigé, la trace réseau a montré un **422** : `hx-include`
manquait, donc le profil ne partait pas. L'input qui le porte vit *à l'intérieur*
du composant, et HTMX n'inclut pas les champs d'un élément qui n'en est pas un.

**Les deux tests e2e de la carte 374 passaient malgré tout.** Ils vérifiaient
`.ks-display`, que le composant met à jour **localement** : l'écran affichait
« Admin » alors qu'aucune requête ne partait. C'est la forme la plus dangereuse
de faux test — il observe précisément la seule chose qui ne dépend pas de ce
qu'il prétend vérifier.

`_choisir_role` attend maintenant la **fin de l'échange** — plus aucun
`.htmx-request` dans la ligne — avant de lire le libellé. Sans quoi le clic
suivant tombe sur un élément en cours de remplacement, le défaut de
`test_dismissals_phase` à l'identique.
