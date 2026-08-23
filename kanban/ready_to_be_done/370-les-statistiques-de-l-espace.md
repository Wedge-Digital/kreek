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

- [ ] Route `SPACE_ADMIN_STATS_WIDGET`, garde `is_admin()`
- [ ] Une seule lecture, les deux compteurs dérivés de la même liste
- [ ] Troisième compteur à zéro, avec un commentaire disant pourquoi
- [ ] Les quatre déclencheurs `hx-trigger`, `memberAdded` compris
- [ ] Racine en `hx-disinherit="*"`
- [ ] Feuille nommée d'après la racine, inscrite dans `FEUILLES_APP`, portée
      vérifiée
- [ ] `make lint`, `make check-arch`, `make test` passent
