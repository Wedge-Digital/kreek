# La liste des membres

**Priorité : haute**
**Dépend de :** 366 et 368 — parallélisable avec 370
**Conception :** `docs/specs/space-admin/membres/04-dtos.md`
**Fichiers :** `io/web/controllers/widgets/space_admin_members_widget.rs`,
`io/web/templates/widgets/{space-admin-members.html, _member-row.html}`,
`assets/static/css/widgets/space-admin-members.css`

## Objectif

Le widget de lecture : recherche, puis une ligne par membre avec avatar, pseudo,
email, sélecteur de rôle et actions. **Aucune mutation** — les deux actions sont
la carte 371.

## Le fragment de ligne existe pour deux appelants

`_member-row.html` est inclus par la liste **et** rendu seul par l'action de
changement de rôle (carte 371). Sans lui, cette action re-rendrait toute la
liste pour une seule ligne.

Le préfixe `_` suit `_coach-result-rows.html`, déjà en place dans le BC.

## `role_locked` et `removable` sont une politesse, pas une garde

```
role_locked = is_self || (is_admin && nombre_d_admins == 1)
removable   = !is_self && !(is_admin && nombre_d_admins == 1)
```

Ils grisent le sélecteur et retirent le bouton, pour que l'interface ne propose
pas ce qu'elle refusera. **La règle qui fait foi vit dans l'agrégat** : un client
qui contourne le grisage se fait refuser par le domaine.

C'est la répartition qu'impose le `CLAUDE.md` — le front grise, le domaine
refuse — et elle se vérifie en test, à la carte 371, par un POST direct.

## La recherche est au front

Filtre Alpine sur les lignes déjà rendues. La liste des membres d'un espace
tient dans un écran ; un aller-retour par frappe n'achèterait rien.

C'est l'inverse du choix de `coach-search`, qui interroge le serveur — mais lui
cherche dans l'annuaire de la plateforme, dont la taille croît sans rapport avec
l'espace. Les deux choix sont justes pour leur liste.

## Ce qui ne se réutilise pas

`space-members-widget` porte presque le nom de ce qu'on construit et fait autre
chose : c'est un **sélecteur** de coachs pour formulaires. Aucune réutilisation.

## Checklist

- [ ] Route `SPACE_ADMIN_MEMBERS_WIDGET`, garde `is_admin()`
- [ ] Une seule lecture — `list_members_with_profile`
- [ ] `MemberRowVm` construit dans `builders.rs`, pas par un `from_domain()` :
      il dépend d'un DTO de port **et** de l'URL rendue par le `host_layout`
- [ ] `is_self` marqué depuis `AuthSession`, jamais depuis le client
- [ ] Email affiché — validé en phase 4
- [ ] `initials` par `crate::common::initials::initials`, comme `coach_search_results`
- [ ] Sélecteur de rôle en **`<kreek-select>`** — les `<select>` natifs sont
      interdits hors maquette
- [ ] Racine en `hx-disinherit="*"`
- [ ] Feuille nommée d'après la racine du widget, inscrite dans `FEUILLES_APP`,
      section widgets, et ne stylant rien au-delà de sa portée
- [ ] `scripts/check-css-collisions.sh` et `debordements.py` passent
- [ ] Scripts scopés par `document.currentScript.previousElementSibling`
- [ ] `make lint`, `make check-arch`, `make test` passent
