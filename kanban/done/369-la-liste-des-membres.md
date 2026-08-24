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

- [x] Route `SPACE_ADMIN_MEMBERS_WIDGET`, garde `is_admin()`
- [x] Une seule lecture — `list_members_with_profile`
- [x] `MemberRowVm` construit dans `builders.rs`, avec quatre tests unitaires
- [x] `is_self` marqué depuis `AuthSession`, jamais depuis le client
- [x] Email affiché
- [x] `initials` par `crate::common::initials::initials`
- [x] Sélecteur de rôle en **`<kreek-select>`**, avec ses options statiques
- [x] Racine en `hx-disinherit="*"`
- [x] Feuille nommée d'après la racine, inscrite dans `FEUILLES_APP` — le verrou
      de l'axe 15 m'a refusé une racine `.sam` alors que la feuille s'appelle
      `space-admin-members.css` : **le nom du fichier est le sélecteur de portée**
- [x] `check-css-collisions.sh` passe
- [x] `_member-row.html` créé, pour la carte 371
- [x] Onglet Membres câblé sur la page hôte
- [ ] ~~`decalages.py` rend 0 px~~ — la mesure rend bien 0 px, **mais elle est
      insensible sur cette page**. Voir ci-dessous : la case n'est pas cochée
      parce que la mesure ne prouve rien, pas parce qu'elle échoue
- [x] `make lint`, `make check-arch`, `make test` passent — 1128 tests

## Ce qu'on a appris en la faisant

**La réservation de hauteur ne corrige pas ce que la carte 368 croyait.**
`decalages.py` rend 0 px sur la page d'administration — et le rendrait tout
autant sans réservation. Rien du flux normal ne se trouve sous la zone
d'onglet : le document se termine par le tiroir mobile et la tabbar, tous deux
en `position: fixed`. La zone grandit vers le bas et n'a rien à pousser.

Ce que la réservation apporte réellement : la hauteur de la page ne s'effondre
pas entre deux onglets de tailles très différentes. Sous une fenêtre déjà
défilée, un panneau plus court ferait remonter la vue d'un coup. `decalages.py`
ne le mesure pas — il compare des positions **dans** la page, pas la position de
défilement.

La justification a donc été corrigée dans la feuille elle-même, plutôt que
laissée fausse dans une case cochée. **Aucune mesure automatique ne protège
cette réservation** : c'est écrit sur place.

**Le verrou de portée CSS m'a repris.** J'avais nommé la racine `.sam` pour une
feuille `space-admin-members.css`. L'axe 15 a refusé vingt-deux règles hors
portée. Le nom du fichier **est** le sélecteur, comme `dis-page.css` avec
`.dis-page`.

**Et un défaut d'écran, corrigé au passage.** L'entrée de menu « Espace » portait
`hx-select="#app-content"`, copié des autres entrées du menu — qui visent des
pages étendant le layout de l'hôte. `spaces` est extractible, ne peut pas
l'étendre, et sa page rend un **fragment nu** sans `#app-content`. HTMX ne
trouvait rien à sélectionner et n'échangeait rien : écran blanc, sans erreur,
sans log. Les deux tests du menu ne l'auraient pas vu — ils vérifiaient que
l'entrée est rendue, elle l'était et ne menait nulle part. L'assertion manquante
a été ajoutée, et vue échouer.
