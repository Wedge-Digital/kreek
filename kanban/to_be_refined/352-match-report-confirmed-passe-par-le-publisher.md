# `match_report` — `MatchReportConfirmed` doit naître d'un domain event

**Priorité : moyenne** — dette architecturale, pas de symptôme utilisateur
**Trouvée par :** carte 350, en inventoriant les sites d'émission d'app events
**Fichiers :** `src/app/match_report/use_cases/create_match_report_use_case.rs`,
`src/app/match_report/use_cases/update_match_selection_use_case.rs`,
`src/app/match_report/io/app_events/app_event_publisher.rs`

## Le problème

`CLAUDE.md` est explicite : l'`app_event_bus` ne vit que dans la couche IO, un
use case n'émet **jamais** d'app event, et tout app event est le résultat d'un
domain event converti par le publisher.

Deux use cases enfreignent la règle, tous deux pour le même app event :

| Use case | Ligne |
|---|---|
| `create_match_report_use_case` | `confirm_draft` publie `MatchReportConfirmed` |
| `update_match_selection_use_case` | publie le même app event, à l'identique |

La cause est visible : **le publisher de `match_report` ne traite pas
`MatchReportConfirmed`.** Son aiguillage ne connaît que `MatchReportPublished`,
`MatchReportUnpublished` et `MatchReportCancelled`. Les deux use cases ont donc
court-circuité la couche IO plutôt que d'ajouter un bras.

Le fait est déjà correctement modélisé côté domaine — `confirm_selection()`
retourne bien un domain event, qui est appendé. Il ne manque que la conversion.

## Ce qu'il faut décider

Le bras du publisher a besoin de `home_team_id`, `away_team_id`, `space_id` et
`pairing_id`. Deux voies, et c'est ce qui reste à trancher :

- **enrichir le domain event** pour qu'il porte ces quatre valeurs — cohérent
  avec ce que fait `MatchReportCancelled`, dont la carte d'origine note que
  « cet app event se construit depuis l'événement lui-même : relire l'agrégat
  ne donnerait qu'un état `Cancelled` qui ne retient plus les équipes » ;
- **relire l'agrégat** dans le publisher, comme le font `handle_published` et
  `handle_unpublished`. L'état après confirmation est `PreMatch`, qui retient
  ces valeurs — à vérifier.

La duplication entre les deux use cases disparaît d'elle-même une fois la
conversion remontée dans le publisher : c'est le même app event construit deux
fois, à l'identique.

## Ce que ça débloque

L'axe 12 de `check-arch` interdit déjà tout `send` direct hors de `publier()`.
Il ne sait pas dire, en revanche, qu'une **couche** n'a pas le droit d'émettre :
une fois cette carte faite, un axe « pas d'`app_event_bus` dans `use_cases/` »
devient posable, et la règle de `CLAUDE.md` cesse d'être une règle sans verrou.

## Terminé quand

- `MatchReportConfirmed` est produit par le publisher, depuis le domain event
- Les deux use cases ne connaissent plus l'`app_event_bus`
- Un axe `check-arch` interdit `app_event_bus` dans `src/app/*/use_cases/`
- Le parcours de confirmation d'un rapport de match est vérifié en e2e
