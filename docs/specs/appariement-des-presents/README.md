# Appariement des présents

Apparier au sort les équipes présentes à une journée, en évitant les
rencontres déjà jouées. Vit dans l'onglet Calendrier de l'administration,
sur une journée sans rencontres.

Reprend l'algorithme de tirage de la branche `sondage-presence`
(`domain/tirage.rs`), sans le sondage par e-mail.

**Maquette :** `assets/rawpages/html/app-competition-admin-schedule-draw.html`

## Décisions de la phase 1

| # | Règle |
|---|---|
| R1 | Les présences ne sont pas mémorisées : la sélection ne sert qu'au tirage. |
| R2 | Toutes les équipes inscrites sont présentes par défaut ; on décoche les absentes. |
| R3 | Le tirage se fait poule par poule. |
| R4 | Deux équipes d'un même coach ne se rencontrent jamais. |
| R5 | Une revanche est permise quand elle est inévitable, et signalée : mieux vaut une revanche qu'une équipe sans match. |
| R6 | L'aperçu ne se retouche pas : on retire au sort, ou l'on corrige après écriture en supprimant un appariement et en l'ajoutant à la main, comme aujourd'hui. |
| R7 | « Générer les rencontres » devient « Générer une journée complète » : le même tirage, parmi toutes les équipes inscrites. |
| R8 | L'exemptée d'une poule impaire est l'équipe qui a joué le plus de matchs. |
| R9 | Le bouton « Faire les appariements » n'apparaît que sur une journée sans rencontres. |
| R10 | Moins de deux présents dans une poule : aucune rencontre n'y est tirée. Le tirage est refusé si aucune poule n'en a deux. |
| R11 | Rien n'est écrit avant la validation, qui revérifie que la journée est toujours vide et que les équipes sont toujours inscrites. |
| R12 | « Générer toutes les rencontres » passe par le même tirage que « Générer une journée complète » (phase 2). |
| R13 | Une rencontre écrite par la confirmation ou la génération est une rencontre ordinaire du Calendrier (phase 3). |
| R14 | « Générer une journée complète » est tout ou rien : un échec d'écriture n'écrit aucune rencontre de la journée (phase 5). |

## Avancement

Spec de la page : `calendrier/`. Épic : `E18`, cartes 595 à 602.

| Page | Maquette | Front | Back | DTOs | Use cases | Domaine | Intégration | Cartes |
|---|---|---|---|---|---|---|---|---|
| Calendrier · appariement des présents | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ |
