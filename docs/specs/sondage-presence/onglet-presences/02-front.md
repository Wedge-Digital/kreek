# Phase 2 — Architecture front : l'onglet Présences

**Entrée** : `assets/rawpages/html/app-competition-admin-presences.html`, validée
en phase 1 (six états).

## Ce que l'existant impose, et qui n'était pas su en phase 1

### L'onglet Calendrier donne le patron, à trois détails près

`templates/admin/schedule.html` compose deux conteneurs `hx-get` qui écoutent
des événements de `body`, et `schedule_widgets.rs` rend les deux fragments.
C'est exactement la forme dont l'onglet Présences a besoin, et sa géométrie est
déjà celle de la maquette.

**L'astuce à reprendre** : `document.body.dataset.activeRoundId`. Le conteneur
de détail écoute deux événements — `roundSelected`, qui porte la journée dans
son `detail`, et `scheduleChanged`, qui n'en porte aucune. Sans mémoire de la
journée courante, toute mutation rechargerait un détail vide :

```html
hx-vals='js:{"round_id": (event && event.detail && event.detail.round_id)
             || document.body.dataset.activeRoundId || ""}'
```

**Les deux détails à ne pas reprendre** sont décrits plus bas, en « Écarts
assumés ».

### `roundSelected` est réutilisé, pas dupliqué

L'événement existe déjà, émis par la barre latérale du Calendrier. Les deux
onglets ne coexistent jamais : `admin-page.html` les échange dans
`#admin-content` en `innerHTML`, donc le markup du précédent a quitté le DOM
avant que le suivant n'écoute.

Le réutiliser a un bénéfice, et c'est la raison du choix : `activeRoundId`
survit au changement d'onglet, donc l'organisateur qui regarde la journée 3 au
Calendrier retrouve la journée 3 en Présences. Un `presenceRoundSelected`
séparé aurait été isolé pour se prémunir d'une collision impossible, en perdant
cette continuité.

### Les deux ports nécessaires existent

| Besoin | Port | Ce qu'il rend |
|---|---|---|
| Les équipes engagées de la saison | `ITeamInfoPort::find_enrolled_teams` | `team_id`, `team_name`, `coach_id`, `coach_name`, `logo_url` |
| L'adresse des coachs | `ICompetitionSpaceMemberPort::list_space_members` | `coach_id`, `coach_name`, `email` |

`TeamInfoDto` **ne porte pas l'adresse** : c'est le croisement des deux listes
sur `coach_id` qui produit les destinataires, et son défaut de correspondance
qui produit le compte « sans adresse connue » de R3. Aucun port à créer, aucun
adapter à écrire.

## Les widgets

| Widget | BC | Endpoint | Trigger | Émet | Mode |
|---|---|---|---|---|---|
| `presence-rounds` | competitions | `GET …/{season_id}/presences/rounds` | `load, presenceChanged from:body` | `roundSelected` | sélection |
| `presence-panel` | competitions | `GET …/{season_id}/presences/panel?round_id=` | `roundSelected from:body, presenceChanged from:body` | — | lecture + mutations |

Le BC `competitions` possède les saisons, les journées et les appariements :
**ni port ni adapter** pour ces deux widgets, hors les deux ports ci-dessus.

### Un seul widget de panneau, pas un par état

Le panneau rend **l'un des six fragments** — aucun sondage, en cours, clos,
tirage proposé, journée appariée, défection — et c'est le serveur qui choisit.

Un widget par état obligerait le navigateur à savoir où en est la campagne pour
demander le bon endpoint : la machine à états serait alors écrite deux fois, une
fois dans le domaine et une fois en JavaScript, et c'est la seconde qui
dériverait. `schedule_round_detail_widget` tranche déjà ainsi entre « repos » et
« journée normale ».

Conséquence à assumer : le panneau est un `match` à six branches côté serveur, et
six gabarits. C'est le prix, et il est payé une fois.

### Aucun widget existant n'est réutilisable

Les trente-sept widgets du projet ont été parcourus. Les plus proches —
`teams/enrolled_teams_widget.rs` et `teams/competition_teams_widget.rs` —
listent bien les équipes engagées d'une compétition, mais appartiennent au BC
`teams` : la **règle 1 des widgets** interdit à `competitions` de les
référencer. Le besoin passe donc par `ITeamInfoPort`, qui existe.

## Les événements

```
presence-rounds  ──roundSelected { round_id }──►  presence-panel
     ▲                                                  │
     └──────────────presenceChanged (HX-Trigger)◄────────┘
```

| Événement | Charge | Émis par | Écouté par |
|---|---|---|---|
| `roundSelected` | `{ round_id }` | la barre latérale, au clic | le panneau |
| `presenceChanged` | aucune | toute action mutante, en `HX-Trigger` | la barre latérale **et** le panneau |

`presenceChanged` ne porte rien : les deux widgets se rechargent, la barre
depuis la saison et le panneau depuis `activeRoundId`. Lui faire porter un état
obligerait à décider *quoi* recharger dans le front, ce qui est la décision que
le serveur vient de prendre.

## Les actions

Toutes en `POST`, toutes rendant `HX-Trigger: presenceChanged` — sauf `draw`.

| Action | Endpoint | Ce qu'elle fait |
|---|---|---|
| `launch` | `…/presences/launch` | ouvre la campagne (échéance, relance auto), expédie les e-mails |
| `answer` | `…/presences/answer` | pose ou change une réponse : `{ round_id, team_id, present }` |
| `remind` | `…/presences/remind` | relance les sans-réponse |
| `close` | `…/presences/close` | clôt la campagne |
| `reopen` | `…/presences/reopen` | la rouvre — R7 réarme les jetons existants |
| `draw` | `…/presences/draw` | **calcule un tirage et rend l'aperçu**, sans rien écrire |
| `confirm-draw` | `…/presences/confirm-draw` | écrit les appariements au calendrier |
| `undo-draw` | `…/presences/undo-draw` | les retire |
| `repair` | `…/presences/repair` | valide la rencontre de remplacement après R12 |

### `draw` est un POST qui n'écrit rien, et c'est délibéré

L'aperçu ne persiste pas : la réponse **est** le fragment, et un rechargement de
page revient au sondage clos. Le verbe reste `POST` parce que le tirage est
aléatoire — donc non idempotent — et qu'un `GET` serait rejoué par le navigateur
au retour arrière, produisant un tirage différent de celui qu'on regardait.

Ce que cela coûte : deux organisateurs simultanés voient deux tirages différents,
et le premier qui valide gagne — le second recevra le refus de R11, la journée
étant désormais appariée. Persister l'aperçu aurait ajouté un état à l'agrégat
pour un conflit qui demande trois administrateurs actifs à la même minute.

## Front et back

**Côté front, presque rien** — et c'est voulu : la page hôte est un assemblage,
comme `schedule.html`.

| Ce qui vit dans le navigateur | Pourquoi |
|---|---|
| la mémoire de `activeRoundId` | quatre lignes, reprises du Calendrier |
| les champs du formulaire de lancement (date, case de relance) | contrôles natifs, envoyés au `POST` |

**Aucun Alpine, aucun calque, aucun état dupliqué.** La correction manuelle
d'une réponse — deux boutons `présent` / `absent` posés sur la carte — est un
`POST` par bouton. C'est le changement que la phase 2 a imposé à la maquette :
elle dessinait un menu au « ⋯ », qui aurait demandé un `x-data` par carte et un
calque qu'un parent en `overflow` peut rogner, pour ajouter un clic à chaque
correction.

## Écarts assumés avec l'onglet Calendrier

### Les refus s'affichent dans le panneau, pas en `alert()`

`schedule.html` expose `window.handleScheduleActionResponse`, qui lit le JSON de
la réponse et lève une boîte du navigateur pour les refus et les avertissements.
L'onglet Présences ne le reprend pas.

La raison tient à la nature de ce qu'il a à dire. « Le tirage a produit une
revanche parce que toutes les paires inédites étaient épuisées » n'est pas une
alerte : c'est une explication, elle appartient à l'aperçu qu'elle commente, et
la maquette l'y place déjà. Une fois cette explication dans le panneau, y mettre
aussi les refus donne **un seul vocabulaire** pour tout ce que l'écran répond.
Le mélange — refus en boîte, explications à l'écran — obligerait à trancher pour
chaque nouveau message de quel côté il tombe, et cette frontière dérive.

Conséquence : les actions rendent un fragment de panneau, pas un JSON. Elles
répondent donc `200` avec le panneau et son motif, là où le Calendrier répond
`4xx` avec `{ "error": … }`.

### Pas de `onclick="fetch(…)"`

`schedule-round-detail.html` porte des attributs `onclick` de plus de quatre
cents caractères, qui reconstruisent à la main un appel HTMX. Les actions de
l'onglet Présences sont des `hx-post` ordinaires avec `hx-swap` sur le panneau.
Ce n'est pas une préférence de style : un `fetch` manuel court-circuite le cycle
htmx, et c'est précisément ce qui a rendu nécessaire le `handleScheduleActionResponse`
ci-dessus.

## Règles métier apparues en phase 2

### R15 — Le tirage refuse en dessous de deux présents, et le dit

`generate_round_pairings` rend une liste vide quand `teams.len() < 2` : sans
garde, l'aperçu s'afficherait vide et l'organisateur croirait à une panne. La
garde reste nécessaire après la correction de R8 — aucun algorithme n'apparie
une équipe seule.

C'est le motif déjà retenu pour `skipped_group_names` dans
`generate_pairings.rs` — *signaler explicitement plutôt que de laisser l'admin
croire que la génération a échoué*. Le bouton « Apparier les présents » est donc
inactif sous deux présents, avec son motif à côté.

### R16 — Une arrivée tardive se traite comme une défection, en sens inverse

R12 décrivait le passage **présent → absent** après le tirage. Le sens inverse
existe autant : un coach annoncé absent qui se libère la veille.

La règle est la même, et R12 se généralise : **toute modification de présence
après le tirage ne refait que les rencontres touchées.** L'arrivant rejoint le
vivier des orphelins ; s'il existe une équipe exemptée, le système propose de
les apparier ; sinon l'arrivant devient lui-même exempté.

Formuler R12 sur la seule défection aurait produit un domaine qui sait retirer
une équipe et pas en ajouter une — et l'asymétrie ne se serait vue qu'à
l'implémentation, quand il aurait fallu inventer un second chemin pour le cas
symétrique.

**C'est la contrainte qui commande la forme de l'agrégat**, avec R12 : le tirage
ne peut pas être une fonction pure `présents → appariements`, puisqu'il doit
savoir refaire *une* rencontre en connaissant les autres.
