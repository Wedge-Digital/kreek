# Calendrier · appariement des présents — architecture front

**Maquette :** `assets/rawpages/html/app-competition-admin-schedule-draw.html`

## L'existant

L'onglet Calendrier assemble deux widgets du BC `competitions` : la liste des
journées et le détail d'une journée, rechargé sur `roundSelected` et
`scheduleChanged`. Les mutations répondent `HX-Trigger: scheduleChanged`, et
`window.handleScheduleActionResponse` (`templates/admin/schedule.html`) rend
leurs refus et avertissements en toasts qui restent jusqu'au clic.

## Les widgets

| Widget | BC | Endpoint | Déclencheur | Émet | Mode |
|---|---|---|---|---|---|
| Détail de journée (existant, modifié) | competitions | `GET …/admin/schedule/round` | `roundSelected`, `scheduleChanged` | — | lecture, mutations existantes |
| **Appariement des présents** (nouveau) | competitions | `GET …/admin/schedule/draw?round_id=…` | clic sur « Faire les appariements » | — | sélection, puis aperçu |

Le nouveau widget **remplace le contenu du détail de journée** sous son
en-tête et ses dates, qui restent visibles.

Ce qui change dans le détail de journée :

- « Générer les rencontres » devient **« Générer une journée complète »**.
- Un bouton **« Faire les appariements »** s'y ajoute.
- Les deux n'apparaissent **que sur une journée sans rencontres** (R9).

## Les actions

| Action | Entrée | Réponse |
|---|---|---|
| `GET …/schedule/draw` | `round_id`, `absent` (facultatif) | fragment « Présents ». `absent` restitue les décochages au retour de l'aperçu. |
| `POST …/schedule/draw/preview` | `round_id`, `team_ids[]` | fragment « Aperçu », swappé à la place du widget. **Rien n'est écrit.** « Retirer au sort » renvoie la même requête. |
| `POST …/schedule/draw/confirm` | `round_id`, couples `home`/`away`, `exempted` | `HX-Trigger: scheduleChanged` et un toast de succès. |
| `POST …/schedule/generate-round` (existant) | `round_id` | inchangé côté front ; l'algorithme change en phase 3. |

La confirmation **revérifie tout côté serveur** (R11) : journée toujours vide,
équipes toujours inscrites, jamais deux équipes d'un même coach, chaque équipe
une seule fois, et dans sa poule. Les étiquettes de revanche sont recalculées
par le serveur, jamais reprises du navigateur.

## Front et back

**Côté navigateur (Alpine, sans appel serveur)** : cocher et décocher, les
compteurs par poule, les avertissements « nombre impair » et « moins de deux
présents », le bouton « Tirer au sort » désactivé quand aucune poule n'a deux
présents.

**Côté serveur** : le tirage, l'aperçu et l'écriture.

## Événements

Aucun nouveau. Le widget réutilise `scheduleChanged`, déjà écouté par la liste
et le détail. « Annuler » recharge le détail de la journée.

## Retours à l'organisateur

| Situation | Rendu |
|---|---|
| Écriture réussie | toast de succès : « 5 rencontres écrites · Plague Bringers exemptée · Dwarf Giants absente » |
| Refus à l'écriture (journée modifiée entre-temps, équipe désinscrite) | toast d'erreur, par `handleScheduleActionResponse` |
| Moins de deux présents dans une poule | note dans la page, calculée par le navigateur |

Le message de succès de la maquette (état 4) devient un toast : `scheduleChanged`
recharge le détail de journée, et une note dans la page disparaîtrait avec lui.

## Conventions

Racine en `hx-disinherit="*"`, feuille `widgets/schedule-draw.css` inscrite
dans `css_bundle.rs` et scopée sous `.schedule-draw`, aucun `id` global, aucun
`style=""`.

## Règles métier ajoutées à cette étape

| # | Règle |
|---|---|
| R12 | « Générer toutes les rencontres » passe par le même tirage que « Générer une journée complète ». |
