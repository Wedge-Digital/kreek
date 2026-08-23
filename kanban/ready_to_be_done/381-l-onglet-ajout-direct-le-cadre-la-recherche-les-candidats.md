# L'onglet Ajout direct : le cadre, la recherche, les candidats

**Priorité : haute**
**Dépend de :** 368 et 377
**Conception :** `docs/specs/space-admin/ajout-direct/{02-front.md, 04-dtos.md}`
**Maquette :** `assets/rawpages/html/app-space-admin.html`, bloc `#tab-direct-add`
**Fichiers :** `io/web/controllers/widgets/space_admin_candidates_widget.rs`,
`io/web/templates/widgets/{space-admin-candidates.html, _candidate-row.html}`,
`assets/static/css/widgets/space-admin-candidates.css`

## Objectif

Le bandeau d'avertissement, le champ de recherche, et la liste des candidats.
**Lecture seule** — l'ajout est la carte 382.

## Le bandeau n'est pas décoratif

« L'ajout direct se passe du consentement du coach. » C'est ce qui justifie que
l'événement trace `added_by`, et que la maquette réserve ce chemin aux coachs
qu'on connaît. Le texte se transcrit tel quel.

## Trois états, pas deux

| État | Rendu |
|---|---|
| moins de deux caractères | « tapez au moins deux caractères » |
| aucun résultat | « aucun coach ne correspond à *xyz* » **et l'invitation à créer un compte** |
| des résultats | la liste, vingt au plus |

Les deux premiers ne disent pas la même chose, et **seul le second propose de
créer un compte**. Les confondre ferait proposer une création dès la première
frappe.

## Le fragment de ligne existe pour deux appelants

`_candidate-row.html` est inclus par la liste **et** rendu seul par l'action
d'ajout de la carte 382. Sans lui, cette action re-rendrait toute la liste pour
une seule ligne.

Une ligne rend soit un sélecteur de profil et un bouton, soit un badge « Déjà
membre » — selon `est_membre`, jamais selon une décision du gabarit.

## Checklist

- [ ] Route `SPACE_ADMIN_CANDIDATES_WIDGET`, garde `is_admin()`
- [ ] Recherche débouncée, `hx-get` avec `hx-params="q"`
- [ ] Les trois états, distincts
- [ ] Seuil et plafond appliqués **côté serveur**, jamais en paramètre
- [ ] `CandidateRowVm` construit dans `builders.rs`
- [ ] Email affiché — décidé en phase 4, avec ses trois garde-fous : seuil,
      plafond, `is_admin()`
- [ ] Sélecteur de profil en **`<kreek-select>`**
- [ ] Racine en `hx-disinherit="*"`, paramètres contextuels **baked** dans l'URL
      par Askama, jamais par `hx-include`
- [ ] Feuille nommée d'après la racine, inscrite dans `FEUILLES_APP`, portée
      vérifiée par `check-css-collisions.sh` et `debordements.py`
- [ ] Aucun `style="…"` — la maquette en contient
- [ ] `make lint`, `make check-arch`, `make test` passent
