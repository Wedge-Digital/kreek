# Saisie des actions — gain de la Haine · Phase 2 : architecture front

**Maquette** : `assets/rawpages/html/app-match-report-step3-haine.html`
**Écran concerné** : `step3` (équipe qui reçoit) et `step4` (équipe visiteuse) —
le même assemblage, servi par `actions_step_controller` avec un `TeamSide`
différent.

## L'assemblage existant

La page hôte `actions-step.html` ne porte aucune logique métier : elle compose
cinq conteneurs et un toast.

| Widget | BC | Endpoint | Trigger | Émet | Mode |
|---|---|---|---|---|---|
| `#turn-selector` | `match_report` | `step3_turn_selector` | `load` | `turnSelected {turn}` | sélection |
| `#player-selector` | **`players`** | `match_player_selector` | `load` | `playerSelected` | sélection |
| `#temp-player-selector` | `match_report` | `step3_temp_players` | `load` | `playerSelected {…}` | sélection |
| `#action-panel` | `match_report` | `step3_action_panel` | `htmx.ajax` de la page hôte, après tour **et** joueur | `actionRecorded` | mutation |
| `#action-log` | `match_report` | `step3_log` | `load, actionRecorded from:body, actionDeleted from:body` | — | lecture |

## La Haine n'ajoute aucun widget

Elle vit **dans `action-panel-widget`**, qui porte déjà `showInjury`,
`showSequel`, `injuryType` et `sequelStat` dans son `x-data`, et construit le
POST dans `submitBlesse()`.

Trois raisons de ne pas en faire un widget à part :

1. **Le panneau est déjà rechargé** à chaque changement de tour ou de joueur. Les
   mots-clefs arrivent avec lui — aucune requête au moment du clic, donc aucune
   latence sur le geste le plus fréquent de l'écran.
2. **Un widget dédié devrait recevoir son contexte d'un autre widget** — le tour,
   le joueur, le type de blessure. La règle 2 des widgets l'interdit : ils ne
   s'appellent pas, ils publient sur `body`. On paierait un aller-retour
   d'événements pour une donnée qui ne bouge pas.
3. **L'union des mots-clefs de l'adversaire est constante pour tout le match.**
   La recharger à chaque blessure serait du trafic pur.

## Ce que le widget reçoit à son rendu

Deux listes, résolues côté serveur, dans cet ordre :

- `opponent_keywords` — les mots-clefs du **roster** adverse, affichés en premier
  sous « Dans le roster adverse » ;
- `other_keywords` — tous les autres, derrière le repli.

Les deux portent `{ uid, label }` : l'`uid` part au serveur, le `label` s'affiche.
**Le tri est fait par libellé**, pas par uid — un coach cherche « Nain », pas
« DWARF ».

Le partage se fait sur le **roster** adverse, et non sur les joueurs réellement
alignés (phase 3) : couvrant, et sans dépendance à la feuille de match. Le
libellé suit — « dans le roster adverse », et non « rencontrés », qu'un poste non
aligné démentirait.

## Ce qui reste front, ce qui part au serveur

**Front, en Alpine, sans aucune requête** : l'ouverture de la section, la
réponse oui/non, le filtre à la frappe, l'ouverture du repli, la sélection du
mot-clef, l'état du bouton de confirmation.

**Serveur** : le POST d'action, inchangé dans sa forme, avec deux champs de plus.

```
POST {post_url}
  turn, player_id, player_type, action_type=BLESSE, injury_type
  [sequel_stat]        — déjà là, si injury_type=SEQUEL
  hate_gained=true|false
  [hate_keyword]       — l'uid, seulement si hate_gained=true
```

L'événement émis ne change pas : `actionRecorded`. Le journal se recharge et
affiche la Haine avec la blessure.

## Les quatre comportements de la section

1. **Elle n'apparaît que sur trois blessures** — Amoché, Blessure Sérieuse,
   Séquelle. La liste est une constante nommée (`PEUT_GAGNER_HAINE`), pas trois
   comparaisons dispersées.
2. **Le bouton de confirmation reste masqué** tant que la question n'est pas
   tranchée, et si la réponse est oui, tant qu'aucun mot-clef n'est choisi. Une
   blessure ne s'enregistre pas en laissant la Haine dans le flou.
3. **Le filtre ouvre le repli de lui-même** quand le mot cherché ne se trouve que
   là. Sans quoi le coach tape « yéti », voit une liste vide, et conclut que le
   mot n'existe pas.
4. **Le mot-clef choisi reste visible** même s'il ne correspond plus au filtre, et
   le groupe du roster adverse disparaît quand il devient vide plutôt que de
   laisser un titre au-dessus du rien.

## Règles métier tranchées

| Question | Décision |
|---|---|
| Un journalier peut-il gagner la Haine ? | **Oui** — utile s'il est engagé ensuite |
| Deux fois le même mot-clef ? | **Autorisé** — aucune vérification, c'est au coach de faire attention |
| Plusieurs Haines différentes ? | **Oui**, sans limite |
| Quand est-elle acquise ? | Par le **mécanisme d'impact de match existant** — `PlayerInjured` traverse déjà, `TeamMatchConcluded` applique à la publication, `TeamMatchImpactReverted` défait à la dépublication |
| Supprimer l'action ? | **Supprime la Haine** |

## Le point dur, renvoyé à la phase 3

**Un journalier n'a pas d'existence hors du rapport de match.** Son
`TempPlayerId` est un ULID généré par `init_temp_players_use_case` à chaque
match — deux matchs consécutifs ne lui donnent pas le même identifiant — et il
n'existe aucun agrégat `players` pour lui.

Aucun mécanisme d'**engagement** d'un journalier n'existe non plus : le
recrutement crée un joueur neuf depuis une ligne de roster, sans lien avec le
temporaire d'un match précédent.

La saisie accepte donc la Haine d'un journalier — décision prise — mais **où
elle est écrite, et ce qui la relie au joueur engagé plus tard, reste à
spécifier**. C'est le sujet de la phase 3, et il touche peut-être une
fonctionnalité qui n'existe pas encore.
