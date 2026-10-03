# Brancher le widget logo sur la fiche équipe

**Priorité : moyenne**
**Dépend de :** carte 510
**Fichiers :** `src/app/teams/io/web/team_detail.rs`,
`src/app/teams/io/web/templates/teams-team-detail.html`

## État actuel

`teams-team-detail.html:19-25` affiche `logo_url` en lecture seule (image si
présent, initiales sinon). `TeamDetailVm` porte déjà `logo_url: Option<String>`
(`team_detail.rs:217`).

## Décision actée avec l'utilisateur

- Droit d'édition : coach propriétaire de l'équipe **ou** admin de l'espace —
  c'est exactement ce que `roster_edit_access_service::peut_modifier_effectif`
  calcule déjà (utilisé par `garde_action_equipe`).
- Visibilité : le widget d'édition n'apparaît **que** pour qui a le droit ;
  les autres visiteurs voient le logo/initiales en lecture seule, comme
  aujourd'hui — pas de formulaire visible, pas de champ désactivé.

## Plan

1. `TeamDetailVm` reçoit un champ `peut_editer_logo: bool`, calculé dans
   `team_detail.rs` via `roster_edit_access_service::peut_modifier_effectif`
   (même appel que celui déjà fait pour l'affichage conditionnel existant —
   vérifier s'il est déjà calculé ailleurs dans ce handler pour ne pas le
   dupliquer).
2. Dans `teams-team-detail.html` :
   - si `vm.peut_editer_logo` : charger le widget `team_logo_widget` via
     `hx-get` (chargement différé, cohérent avec le pattern « page hôte
     assemble, widget se charge lui-même »név rien à câbler de plus ici) ;
   - sinon : conserver l'affichage actuel tel quel, inchangé.

## Ce que la carte ne fait pas

- Elle ne modifie pas `peut_modifier_effectif` ni la garde de route.
- Elle ne change pas l'affichage pour qui n'a pas le droit.

## Checklist

- [ ] `TeamDetailVm.peut_editer_logo` calculé sans dupliquer un appel existant
- [ ] Widget chargé conditionnellement dans le template
- [ ] Affichage lecture seule inchangé pour un tiers sans droit
- [ ] Test e2e : le propriétaire voit et utilise le widget, un tiers ne le voit pas
- [ ] `make lint`, `make check-arch`, `make test`, `make e2e` (ou `make test-impacted`)
