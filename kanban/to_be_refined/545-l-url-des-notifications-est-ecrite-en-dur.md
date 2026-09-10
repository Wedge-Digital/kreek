# L'URL des notifications est écrite en dur dans un use case

**Priorité : basse — dette localisée, sans symptôme aujourd'hui**
**Épic :** aucune (dette technique)
**Fichiers :** `src/app/competitions/use_cases/send_due_notifications_use_case.rs`,
`src/app/competitions/use_cases/send_registration_open_use_case.rs`

## Le constat

Les deux déclencheurs de notification composent l'URL de la compétition à la
main, dans la couche applicative :

```rust
competition_url: format!(
    "{}/app/{}/competitions/{}/{}",
    deps.dispatch.app_url, c.space_id, c.competition_id, c.season_id
),
```

`AppRoutes::default().competitions.competition_detail(space, competition, season)`
rend exactement ce chemin. Le `format!` en est une **copie manuelle**, qui ne
suivra pas un renommage de route.

## Pourquoi ce n'est pas urgent

Rien n'est cassé : le chemin est juste, et `COMPETITION_DETAIL` n'a pas bougé.
L'axe 10 de `check-arch` cherche les placeholders substitués par des littéraux
dans les **routes**, pas les chemins reconstruits ailleurs ; l'axe 4 ne regarde
que le front. Personne ne le verra donc avant le jour où la route change — et ce
jour-là, quatre e-mails partiront avec un lien mort, sans qu'aucun test ne
rougisse.

## Ce qu'il faut faire

Passer les URL en paramètre depuis l'appelant qui possède `AppRoutes`, comme
`EtiquettesCampagne` le fait pour le sondage de présence (carte 527). Les deux
déclencheurs étant lancés par la CLI et par un listener, et non par un handler,
il faut décider **qui** joue ce rôle : la CLI peut composer, ou bien un petit
service de liens vivant dans la couche web.

## Question ouverte

Un use case n'a pas de `AppRoutes` par construction — la couche web les possède.
Faut-il injecter un fabricant de liens dans les deux déclencheurs, ou accepter
que la composition se fasse au point d'entrée (CLI, listener) et voyage en
paramètre ? La seconde tient sans nouvelle abstraction, mais duplique l'appel à
deux endroits.
