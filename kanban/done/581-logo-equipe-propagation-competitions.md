# Le nouveau logo atteint les pages de compétition

**Priorité : haute**
**Dépend de :** carte 509 (le use case émet `LogoChanged`)
**Origine :** revue de la PR #11, point 2
**Fichiers :** `src/app/shared_kernel/app_events/teams_app_events.rs`,
`src/app/teams/io/app_events/app_event_publisher.rs`,
`src/app/competitions/io/app_events/team_logo_changed_listener.rs` (nouveau),
`src/app/competitions/context.rs`

## Le problème

`competitions` copie le logo de chaque équipe dans
`competition_match_display_proj` (`home_logo_url` / `away_logo_url`) au moment
de l'appariement. `LogoChanged` ne sortait pas de `teams` : après un
changement de logo, le calendrier, les résultats et la liste des matchs d'une
équipe gardaient l'ancien — ou un logo que le coach avait retiré.

Avant les cartes 508 à 511, rien n'émettait `LogoChanged`, et le défaut ne
pouvait pas se produire. C'est l'édition du logo qui le rend possible.

## La solution

- `TeamsAppEvent::LogoChanged { team_id, space_id, logo_url: Option<String> }`
- le publisher de `teams` le fait sortir (retiré de la liste `=> None`)
- un listener cross-BC dans `competitions` réécrit le côté de l'équipe,
  domicile ou extérieur, dans chacun de ses matchs. `None` remet à NULL.

## Terminé quand

Un coach change le logo de son équipe : le calendrier de sa compétition
affiche le nouveau logo sans réappariement. Il le retire : ses initiales
reviennent.

## Tests

- unitaires (`sqlx::test`) : remplacement des deux côtés, retrait → NULL,
  matchs d'autres équipes intacts, filtre sur le type d'événement
- e2e : scénario « logo visible dans le calendrier » (cf. carte 512)

## Hors périmètre

`TeamRenamed` et `InitialsChanged` ont la même dette — la projection garde
l'ancien nom et les anciennes initiales. Ils relèvent d'un autre chantier que
l'édition du logo : à traiter dans une carte à part.
