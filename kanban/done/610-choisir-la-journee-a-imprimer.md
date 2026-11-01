# Choisir la journée du match à imprimer

**Priorité : moyenne**
**Épic :** E16 — Imprimer la feuille d'équipe
**Dépend de :** 599
**Origine :** revue de la PR #12 — « un match oublié en J3 l'emporte sur la J6 en cours ; est-ce voulu ? »
**Spec :** `06-domaine.md` (R12), `02-front.md`
**Fichiers :** `src/app/competitions/domain/next_opponent.rs`, `src/app/competitions/use_cases/next_opponent_service.rs`, `src/app/competitions/io/web/widgets/print_next_opponent_widget.rs`, `templates/widgets/competitions-print-next-opponent.html`, `src/app/teams/io/web/templates/teams-print-page.html`

## Objectif

Le prochain adversaire reste celui de R12 : le match en cours de saisie,
sinon le premier match à venir dans l'ordre des journées. Un match oublié
d'une journée passée est donc bien le prochain à jouer — mais ce n'est pas
forcément celui que le coach veut imprimer. Il peut choisir une autre journée.

## Conception

- **Domaine** `upcoming_opponents(team, pairings) -> Vec<NextOpponent>` : tous
  les matchs non joués, dans l'ordre de R12. `next_opponent` en est le premier
  élément : une seule règle.
- **Service** : `NextOpponentLookup::Found { upcoming }`, jamais vide, chaque
  match avec sa rencontre.
- **Fragment** : avec plusieurs matchs à venir, un `<kreek-select>` à options
  statiques sous le bouton radio, présélectionné sur le prochain match. Pas de
  route JSON : la liste est rendue avec l'option, et aucune nouvelle URL n'a à
  être contrôlée par `space_scope`.
- **Page** : au `change` du sélecteur, le script recopie l'équipe adverse, la
  journée et le camp sur le bouton radio, puis recharge l'aperçu. L'équipe
  adverse part dans le chemin, contrôlée comme avant (carte 594).

## Checklist

- [x] `upcoming_opponents` + test : ordre de R12, matchs joués écartés, premier = `next_opponent`
- [x] Service + test : chaque match garde sa journée
- [x] Fragment, gabarit, script de la page
- [x] Test e2e : choisir une autre journée change la fiche adverse
- [x] `make check-arch`, `make lint`, `make test`
