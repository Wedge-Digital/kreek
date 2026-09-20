# Déplacer un match sur une autre journée

**Priorité : haute — la seule réparation possible aujourd'hui est une session SQL sur la production**
**Épic :** aucune — une fonction d'administration, livrable d'un bloc
**Dépend de :** 551, 552, 556
**Fichiers :**
`src/app/competitions/domain/match_day.rs`, `domain_event.rs`,
`src/app/competitions/use_cases/admin/move_pairing_use_case.rs` *(nouveau)*,
`src/app/competitions/io/repository/match_day_repository.rs`,
`src/app/competitions/io/web/widgets/move_pairing_widget.rs` *(nouveau)*,
`src/app/competitions/io/web/templates/widgets/move-pairing-widget.html` *(nouveau)*,
`src/app/match_report/domain/events.rs`, `match_report_state.rs`,
`src/app/match_report/use_cases/reassign_round_use_case.rs` *(nouveau)*,
`src/app/match_report/io/app_events/pairing_moved_listener.rs` *(nouveau)*,
`src/app/match_report/io/web/recap_controller.rs`, `templates/recap.html`,
`src/app/players/domain/events.rs`, `player.rs`, `use_cases/match_history_service.rs`,
`src/app/ranking/io/app_events/match_report_round_reassigned_listener.rs` *(nouveau)*,
`src/app/shared_kernel/app_events/*.rs`,
`assets/static/css/pages/ms-page.css`, `assets/static/css/widgets/move-pairing.css` *(nouveau)*,
`tests/e2e/test_deplacer_un_match.py` *(nouveau)*, `tests/impact-map.toml`

## L'objectif

Un administrateur de l'espace ou de la compétition déplace un match et son
rapport d'une journée à une autre, depuis la page du rapport. Tout ce qui cite
la journée suit : calendrier, résultats, classement, historique des joueurs.

La page gagne au passage une **barre d'administration** sous le cartouche,
avant les moments clés, qui accueille toutes les actions réservées : « Corriger
le rapport », qui vivait en bas de page, et « Déplacer sur une autre journée ».

## Ce qui l'a fait naître

Sur la copie de production du 20 septembre, le match Olds Giants contre Lolth
pervertium de Bordeaux, publié 1 à 0 le 2 septembre, est rangé sur la Finale du
23 juin 2027. Le corriger demandait une session SQL sur la production, à sept
endroits dans quatre BCs, sans test.

Une commande `make` a été envisagée et écartée : l'outil ne serait utilisable
que par qui a les identifiants de la base, et son code ne passerait par aucun
agrégat. Un bouton dans l'application permet à n'importe quel administrateur de
corriger l'erreur, et le chemin est testé.

## Ce qu'un déplacement touche

Une journée est citée à sept endroits, dans quatre BCs, pour un match publié :

| BC | Où | Quoi |
|---|---|---|
| competitions | `competition_match_day_pairings` | la journée de l'appariement |
| competitions | `competition_match_display_proj` | la journée et ses six colonnes recopiées |
| match_report | `match_report_proj` | la journée |
| match_report | `match_report_event_store` | `round_id` de `MatchReportCreated`, **par un nouvel événement** et non par réécriture |
| players | `players_events` | `round_id` et `round_label` du contexte de `MatchConcluded`, ce que lit l'historique d'un joueur |
| ranking | `ranking_lines` | la journée de chaque ligne du rapport |

Deux endroits citent la journée et ne bougent pas : `event_log`, journal
d'audit des app events, qui raconte ce qui s'est passé ; et les envois de
notifications, déjà partis.

## Le changement

**`competitions` porte la décision.** Les journées et les appariements lui
appartiennent, et l'invariant « une équipe joue une fois par journée » vit
dans `MatchDay` depuis la carte 551. `MatchDay::accueillir` refuse en nommant
le match qui bloque ; le use case `move_pairing` vérifie que les deux journées
sont de la même saison, persiste dans une transaction, émet `PairingMoved`.

**`match_report` suit l'appariement.** Un `pairing_moved_listener` retrouve le
rapport par son appariement — fiable depuis la carte 556 — et lui applique
`RoundReassigned`, porté par les quatre états vivants. Le publisher le convertit
en `MatchReportRoundReassigned` pour `ranking` et en un `TeamMatchRelocated`
par équipe pour `players`, **seulement si le rapport est publié** : avant
publication, ces deux BCs ne savent rien du match.

**`players` ne réécrit pas ses événements.** L'historique d'un joueur se
reconstruit à la lecture, et le libellé de journée vient du contexte de
`MatchConcluded`. Un `MatchRelocated` par joueur qui a conclu ce match,
appliqué par-dessus dans `build_match_history`. C'est la mécanique des
événements existants, sans l'entorse à l'immuabilité que `m003` a dû assumer.

**`ranking` change une colonne.** La journée d'une ligne n'entre dans aucun
calcul.

**La page du rapport.** La barre d'administration remplace la zone du bas de
`recap.html`. Le déplacement n'y figure que si `est_administrateur` répond oui
— le prédicat de la carte 550, admin d'espace ou de compétition, sans les
coachs. C'est un **widget de `competitions`**, chargé par la page :
`kreek-select` des journées cibles, dans l'ordre du calendrier, sans la
courante ni celles où l'une des deux équipes est déjà engagée. La confirmation
passe par le toast global (carte 553) ; le refus aussi, persistant.

**Ce que le déplacement ne bloque pas.** « Corriger le rapport » est grisé
quand une équipe a dépensé ses SPP. Le déplacement reste possible : il ne
touche ni aux SPP, ni à la trésorerie, ni aux effets du match.

**Le droit de corriger ne change pas.** La dépublication était et reste
ouverte à qui peut voir un rapport publié — les deux coachs compris, c'est
leur match. La barre n'est donc pas « réservée aux administrateurs » : elle
existe dès qu'il y a une action à offrir, et chaque action porte son propre
droit. Un coach y voit « Corriger le rapport » seul ; un administrateur y voit
aussi le déplacement.

**Un en-tête HTTP n'est pas de l'UTF-8.** Le premier toast disait « Match
dÃ©placÃ© en J1 » : le navigateur lit `HX-Trigger` en Latin-1. Le JSON de
l'en-tête est échappé en `\uXXXX`, la seule forme qui traverse intacte. Les
trois `showToast` de `team_creation` ont le même défaut, hors périmètre.

## Ce que la carte ne couvre pas

Les rapports en cours de saisie, étapes 1 à 5, qui n'ont pas de zone
d'administration. Et les appariements en double de GBLR Championnat J11, qui ne
sont pas un déplacement mais un doublon à trancher par l'organisateur.

En passant : `find_pairing_id` ne teste toujours qu'un sens du couple
domicile/extérieur, à rebours de la règle que la doc de `m003` énonce. Sans
effet depuis que la carte 556 a rattaché tous les rapports vivants à leur
appariement, mais l'entorse reste.

## Tests

Unitaires : l'invariant sur `MatchDay` avec le match bloquant nommé ;
`rehydrate` avec `RoundReassigned` sur chaque état ; `build_match_history`
avec `MatchRelocated` ; le use case de déplacement sur un faux dépôt.

E2E, `test_deplacer_un_match.py` : un administrateur déplace un match publié
et voit le cartouche passer de J1 à J2, avec les sept écritures vérifiées en
base ; un coach voit la correction mais pas le déplacement, et le widget lui
est refusé ; le déplacement vers une journée où une équipe joue est refusé
avec le nom du match.

## Terminé quand

Sur la page du rapport de Bordeaux, un administrateur choisit J1 dans la barre
d'administration, voit le toast, et après rechargement le cartouche dit « Ligue
Open · Saison 10 · J1 ». Le calendrier de J1 porte le match, la Finale ne le
porte plus, et l'historique des joueurs des deux équipes dit J1.
