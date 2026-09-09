# Les neuf actions

**Priorité : haute — sans elles l'écran est en lecture seule**
**Épic :** E16 — Sondage de présence
**Dépend de :** 520, et les use cases 515 à 518
**Fichiers :** `src/app/competitions/io/web/admin/presences_actions.rs`,
`src/app/competitions/routes.rs`, `src/app/competitions/router.rs`

## Cette carte déclare et branche ses neuf routes

La 519 n'a posé que les trois routes qu'elle sert. **Les neuf de cette carte lui
appartiennent** — constante de chemin, méthode de génération d'URL, et entrée de
routeur, dans le même commit que leur handler :

```
presences/launch   presences/answer   presences/remind
presences/close    presences/reopen   presences/draw
presences/confirm-draw   presences/undo-draw   presences/repair
```

Une route déclarée sans handler ne compile pas ; une route branchée sur un
handler vide est une porte ouverte sans garde. Les annoncer d'avance aurait
demandé neuf handlers factices, qui passeraient `check-arch` et l'axe 4 sans que
rien ne les appelle.

**Chacune porte `require_admin_access` puis `journee_de_la_saison`**, y compris
sur un POST qui ne rend qu'un fragment : `space_scope` n'a pas de résolveur pour
`round_id`, qui passerait librement (carte 416).

## L'objectif

Neuf handlers POST, et le protocole de réponse qui vaut pour les neuf.

## Le protocole — ce que rend une action

| Cas | Réponse HTTP |
|---|---|
| succès | corps vide + `HX-Trigger: presenceChanged` |
| refus métier (R11, R13, R15, R19, R21, R22) | le fragment de panneau porteur du motif, + `HX-Retarget: #presence-panel` + `HX-Reswap: innerHTML`, **sans** trigger |
| `draw` | le fragment d'aperçu, `HX-Retarget` + `HX-Reswap`, sans trigger |
| corps mal formé, VO invalide | `400` — refus de format, pas refus métier |
| panne | `500` + `tracing::error!` |

Les boutons portent `hx-swap="none"` : le succès ne remplace rien, les deux
widgets se rechargent d'eux-mêmes sur `presenceChanged`. **C'est le serveur qui
redirige le swap** vers le panneau quand il a quelque chose à y dire — mécanisme
htmx standard, pas de JavaScript.

Rendre le fragment *et* déclencher le rechargement peindrait le panneau deux
fois ; rendre un corps vide ne laisserait nulle part où poser le refus.

**`draw` ne déclenche pas `presenceChanged`**, et c'est ce qui le rend possible :
l'aperçu ne persiste rien, un rechargement du panneau le perdrait aussitôt. Le
verbe reste `POST` parce que le tirage est aléatoire — donc non idempotent — et
qu'un `GET` serait rejoué au retour arrière, produisant un tirage différent de
celui qu'on regardait.

## Aucune `alert()`, aucun JSON d'erreur

Écart assumé avec le Calendrier, qui expose `window.handleScheduleActionResponse`
et lève une boîte du navigateur. « Le tirage a produit une revanche parce que
toutes les paires inédites étaient épuisées » n'est pas une alerte : c'est une
explication, elle appartient à l'aperçu qu'elle commente. Une fois cette
explication dans le panneau, les refus y vont aussi — sinon il faudrait trancher
pour chaque nouveau message de quel côté il tombe, et cette frontière dérive.

**Pas de `onclick="fetch(...)"` non plus.** `schedule-round-detail.html` porte des
attributs `onclick` de plus de quatre cents caractères qui reconstruisent un
appel HTMX à la main — c'est précisément ce qui a rendu nécessaire le
`handleScheduleActionResponse` ci-dessus. Les actions sont des `hx-post`
ordinaires.

## Les DTOs d'entrée

Plats, en JSON, convertis par le handler : **DTO plat -> `try_new` -> commande
typée**. Le refus de validation est un `400`, jamais un refus métier.

| DTO | Champs |
|---|---|
| `LaunchBody` | `round_id`, `deadline`, `auto_remind` |
| `AnswerBody` | `round_id`, `team_id`, `presence` |
| `SurveyIdBody` | `round_id` — remind, close, draw, undo-draw |
| `ReopenBody` | `round_id`, `deadline` (R23) |
| `ConfirmDrawBody` | `round_id`, `rencontres`, `exemptee` |
| `RepairBody` | `round_id`, `rencontre`, `exemptee` |
| `PairBody` | `home_team_id`, `away_team_id` |

**La campagne est désignée par `round_id`, jamais par `survey_id`** : R2
garantit une seule campagne vivante par journée, et le client n'a pas à
connaître un identifiant qu'il ne lit nulle part.

**`PairBody` ne porte pas `historique`** : le client n'a pas à renvoyer un motif
que le serveur recalcule de toute façon. Le lui faire porter autoriserait à
mentir sur l'historique d'une rencontre.

## Checklist

- [ ] Les neuf handlers, `require_admin_access` + `journee_de_la_saison` sur les neuf
- [ ] Les sept DTOs, la conversion par smart constructors
- [ ] Le protocole : trois helpers — succès, refus, aperçu
- [ ] Les boutons des gabarits en `hx-swap="none"`
- [ ] Chaque refus métier rend son motif dans le panneau
- [ ] `make lint`, `make check-arch`, `make test`
