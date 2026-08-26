# Onglet Paramètres · Phase 2 : architecture front

**Maquette** : `assets/rawpages/html/app-competition-admin-modification.html`

## L'onglet n'existe pas encore

`admin_page.rs` aiguille cinq onglets par un `match active_tab` — `dashboard`,
`summary`, `enrollments`, `groups`, `schedule`, `results`. **Il n'y a pas de
`settings`** : l'ancienne maquette de paramètres n'a jamais été implémentée.

Après cette fonctionnalité, l'aiguillage compte quatre branches : `summary`
(défaut), `enrollments`, `groups`, `schedule`, `settings`.

## Un onglet d'assemblage, cinq widgets

Le `CLAUDE.md` réserve le pattern d'assemblage aux pages de **trois sections
interactives et plus**. Il y en a cinq ici, chacune avec sa propre mutation et
son propre bouton d'enregistrement.

Le précédent est l'onglet Poules, dont le fragment ne fait que composer deux
conteneurs (`groups.html:21` et `:27`).

| Widget | Endpoint | Trigger | Émet | Mode |
|---|---|---|---|---|
| `#settings-general` | `admin_settings_general` | `load` | — | édition |
| `#settings-ranking` | `admin_settings_ranking` | `load` | — | édition + recalcul |
| `#settings-pools` | `admin_settings_pools` | `load` | — | édition |
| `#settings-tiers` | `admin_settings_tiers` | `load` | — | édition |
| `#settings-visibility` | `admin_settings_visibility` | `load` | — | édition |

**Aucun événement DOM entre widgets, et c'est délibéré.** Les cinq réglages sont
indépendants : aucun ne change ce qu'un autre affiche. Chaque widget se recharge
**lui-même** en réponse à son POST (`hx-swap="outerHTML"` sur sa propre racine),
ce qui affiche l'état enregistré sans toucher au reste de la page.

Le seul effet à distance — modifier les poules vide la répartition — concerne un
**autre onglet**, qui se recharge de toute façon à sa prochaine ouverture.

## Ce qui reste front

**Les états de conséquence.** Le pied de panneau qui passe du gris à l'accent, le
décompte des changements, le libellé du bouton, la poule barrée qu'on peut
rétablir : tout cela est de l'état d'écran, sans aller-retour.

**Rien d'autre.** Pas de calcul de points, pas de validation de bornes — le
serveur refait tout.

## Les deux widgets qui ne sont pas ordinaires

### `#settings-ranking` — le seul dont l'enregistrement est long

Le POST déclenche un recalcul de tout le classement de la saison. Le bouton doit
donc porter un état d'attente (`hx-indicator`), et le widget revenir avec le
résultat.

**À trancher en phase 3** : si le recalcul dépasse la seconde sur une saison
chargée, le POST synchrone ne tient plus et il faudra une autre forme —
traitement en arrière-plan, ou verrou d'écran. On ne le décide pas ici, mais on
sait déjà que c'est le seul endroit qui peut le demander.

### `#settings-pools` — l'état retiré vit dans le formulaire

Une poule marquée « à retirer » n'est **pas** supprimée côté serveur : elle est
barrée dans le formulaire, et le POST porte la liste finale. Tant qu'on n'a pas
enregistré, rien n'est défait — et « Rétablir » n'est qu'un changement d'état
local.

C'est ce qui permet de montrer la conséquence — « 6 équipes à réaffecter » — sans
l'avoir déjà provoquée.

## Deux composants repris, non réécrits

**Le sélecteur de coups de pouce** est le widget de `references`
(`inducement-picker.html`), déjà utilisé par la page de création. Il émet
`inducementPickerChanged` sur `body` avec son `instanceId` et sa sélection —
c'est ainsi que le widget des tiers récupérera les choix, un `instanceId` par
tier.

**C'est la seule communication par événement de cet onglet**, et elle est
interne au widget des tiers : elle existe déjà, on ne l'invente pas.

**Les blocs de tier** reprennent le découpage et les couleurs de
`pages/new-competition-phase-2.css` — en-tête teinté, badge coloré, corps blanc.

## Ce que la page ne fait pas

- **Aucune modification de roster, de budget ni d'XP de départ.** Ils s'affichent
  en libellé-valeur, sans champ : ce ne sont pas des réglages désactivés en
  attendant mieux, ils sont hors sujet dans cette version.
- **Aucun ajout ni retrait de tier.**
- **Aucune gestion des administrateurs** — le panneau de l'ancienne maquette est
  hors demande.

## Règles métier tranchées

**Un classement publié change directement.** Pas d'annonce, pas de gel, pas de
version intermédiaire : le nouveau barème s'applique, le classement se recalcule,
et c'est celui-là que les coachs voient. La conséquence est annoncée **avant**
l'enregistrement, dans le pied du panneau — c'est le seul avertissement, et il
suffit.

**Retirer une poule ne touche aucun résultat ni aucun point.** Les matchs ne sont
pas limités aux membres d'une poule : ils restent joués, quelle que soit la
répartition. C'est vérifiable dans le schéma — `ranking_lines` **ne porte aucune
colonne de poule** : le classement est tenu par saison, et le regroupement par
poule n'est qu'un filtre d'affichage sur l'assignation courante.

Une poule retirée ne défait donc que sa **répartition**. Les équipes deviennent
non assignées, leurs points restent.

**Le classement appartient à `ranking`, les réglages à `competitions`.**

Les cinq panneaux écrivent tous dans `competitions` — le nom, le logo, la
saison, la visibilité, les poules, les tiers, **et le barème**, qui vit dans
`competition_seasons.rules`. Sur ce point, l'écran est bien mono-BC.

Le **recalcul**, lui, écrit dans `ranking_lines`, qui appartient à `ranking`.
Celui-ci consulte déjà les règles par un port —
`IRankingCompetitionPort::find_ranking_rules(season_id)` — sans que
`competitions` ne connaisse le classement.

Le lien est donc un **app event** : `competitions` publie que le barème a changé,
`ranking` recalcule. C'est le critère du `CLAUDE.md` — on ne consulte pas, on
propage un effet résultant de la mutation d'un agrégat. Et ça garde les deux BCs
comme ils sont : celui qui possède les règles ne sait pas ce qu'un classement est.

## Ce qui reste à préciser en phase 3

- **Qui peut modifier ?** L'administration de la compétition est déjà gardée ;
  reste à savoir si tous ses administrateurs ont accès à tous les panneaux.
- **La forme du recalcul** : synchrone au POST, ou déclenché par l'app event et
  mené en tâche de fond ? Le second découple, mais l'écran ne peut alors pas
  confirmer que c'est fait.
