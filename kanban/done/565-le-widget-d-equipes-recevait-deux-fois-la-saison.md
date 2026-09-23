# Le widget d'équipes recevait deux fois la saison

**Priorité : moyenne — en édition, les équipes ne se rechargent plus**
**Épic :** aucune — une correction, livrable d'un bloc
**Dépend de :** rien
**Fichiers :**
`src/app/match_report/io/web/templates/match-selection.html`,
`tests/e2e/test_match_selection_edition.py`,
`tests/impact-map.toml`

## L'objectif

En phase 1 d'un rapport existant, choisir une compétition, une saison ou une
journée recharge le widget d'équipes sans erreur.

## Ce qui l'a fait naître

En production, à l'ouverture de la phase 1 en édition, une requête échoue :

```
/app/…/team/widgets/team-selection?season_id=X&selected_home=…&selected_away=…&season_id=X
```

La saison y figure deux fois. `Query<TeamSelectionWidgetQuery>` refuse un
champ en double (`duplicate field season_id`) : **400**.

## La cause

En édition, `team_widget_url` porte déjà `season_id`, `selected_home` et
`selected_away`. Sur `matchContextSelected`, `handleContext` rechargeait le
widget en **ajoutant** `&season_id=…` à cette URL, au lieu de remplacer la
valeur.

## Pourquoi « parfois »

À l'ouverture, la journée pré-sélectionnée n'émet `matchContextSelected` que si
son écouteur `seasonSelected` est branché **avant** que la saison ne
s'annonce. C'est une course : en local elle est perdue et rien ne part ; en
production, la latence la fait parfois gagner.

Choisir une saison ou une journée, en revanche, émet l'événement à coup sûr.
En édition, **changer de saison ne rechargeait donc jamais les équipes** — la
conséquence qui compte, au-delà de la ligne rouge dans la console.

## Le changement

`handleContext` construit l'URL par `new URL(...)` et
`searchParams.set('season_id', …)` : la valeur présente est remplacée, les
équipes pré-sélectionnées sont conservées.

## Tests

`test_le_widget_d_equipes_ne_recoit_la_saison_qu_une_fois` choisit une saison
— **le geste, pas le chargement**, puisque le chargement dépend de la course —
et exige que la requête du widget ne porte la saison qu'une fois et réponde 200.

Éprouvé contre l'ancien gabarit : il échoue, sur l'URL exacte constatée en
production. Une première version, qui écoutait le chargement, passait avec ou
sans le correctif.

Le test est inscrit dans `tests/impact-map.toml`.

## Terminé quand

En phase 1 d'un rapport existant, choisir une autre saison recharge la liste
des équipes, et aucune requête vers `team-selection` ne répond 400.
