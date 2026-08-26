# Saisie des actions — gain de la Haine · Phase 3 : architecture back

**Entrée** : `02-front.md` validé.

## Widgets → BCs

Aucun widget n'est créé ni déplacé. Le seul touché est `#action-panel`, fourni
par `match_report`, qui n'appelle **aucun port** aujourd'hui — son handler
commence par `let _ = state;`.

| Widget | BC | Change ? |
|---|---|---|
| `#turn-selector` | `match_report` | non |
| `#player-selector` | `players` | non |
| `#temp-player-selector` | `match_report` | non |
| `#action-panel` | `match_report` | **oui** — reçoit les mots-clefs, et deux champs de plus au POST |
| `#action-log` | `match_report` | affichage de la Haine dans la ligne de blessure |

## Ce qui existe et qu'on ne recrée pas

| Besoin | Existant |
|---|---|
| Positions du roster adverse | `ITeamDataPort::find_roster_positions(team_id) -> Vec<RosterPositionDto>` ; l'adapter lit déjà `ref_team.available_players` |
| Trajet vers `players` | `PlayerInjured { context, injury_type }` → appliqué à la publication par `TeamMatchConcluded`, défait par `TeamMatchImpactReverted` |
| Stockage d'une compétence acquise | `players_proj.acquired_skills`, JSONB, via `AcquiredSkillProjection` |
| Refus d'achat en SPP | `resolve_skill_cost` rend `CategoryNotAccessible` — la catégorie `TRAITS` n'est dans l'accès d'aucun poste |

## Plan de fichiers

### `references` — le corpus

| Fichier | Change |
|---|---|
| `assets/references.example/keywords_fr.json` | **nouveau** — `{ keywords: [{ uid, label }] }`, même forme que `special_rules_fr.json` |
| `domain/models.rs` | `PlayerPosition` gagne `keywords: Vec<String>` en `#[serde(default)]` ; nouveau `Keyword { uid, label }` |
| `io/repository/in_memory_reference_repository.rs` | chargement du fichier au démarrage, à côté des onze autres |
| `domain/port.rs` | `list_keywords(&self) -> &[Keyword]`, `find_keyword_by_uid(&self, uid: &str) -> Option<&Keyword>` |

`#[serde(default)]` sur `keywords` : sans lui, tout corpus non migré cesserait de
charger, et `load_references` refuse de démarrer sur une erreur de
désérialisation. Le champ absent doit donner une liste vide, pas un serveur mort.

### `match_report` — le port et le service

| Fichier | Change |
|---|---|
| `ports.rs` | `RosterPositionDto` gagne `keywords: Vec<String>` ; nouveau port `IKeywordCatalogPort` |
| `src/infrastructure/match_report/keyword_catalog_adapter.rs` | **nouveau** — implémente le port depuis `IReferenceRepository` |
| `src/infrastructure/match_report/ref_team_data_adapter.rs` | remplit `keywords` dans `find_roster_positions` |
| `use_cases/hate_keywords_service.rs` | **nouveau** — domain service : partage le catalogue en deux listes triées |
| `io/web/widgets/action_panel_widget.rs` | appelle le service, porte les deux listes dans son template |
| `io/web/templates/action-panel-widget.html` | la section, le filtre, le repli |
| `io/web/record_action_controller.rs` | lit `hate_gained` et `hate_keyword` |
| `domain/value_objects.rs` | `MatchActionType::Blesse` gagne `hatred: Option<HatredKeyword>` |
| `context.rs`, `src/main.rs` | câblage du nouveau port |

**Pourquoi un port dédié plutôt qu'une méthode de plus sur `ITeamDataPort`** :
un catalogue de mots-clefs n'est pas une donnée d'équipe. `ITeamDataPort` répond
« que sait-on de cette équipe ? » ; `IKeywordCatalogPort` répond « quels
mots-clefs le règlement connaît-il ? ». Les fondre obligerait à passer un
`team_id` à une question qui n'en a pas.

**Le domain service est obligatoire ici** : sans lui, le handler manipulerait
`RosterPositionDto` et le DTO du catalogue pour en faire des VMs — ce que le
`CLAUDE.md` interdit explicitement (« Domain services pour données inter-BCs »).
Il rend deux listes de mots-clefs du domaine, triées par libellé, partagées entre
« chez l'adversaire » et « les autres ».

### `players` — l'écriture

| Fichier | Change |
|---|---|
| `domain/player.rs` | `AcquisitionMode` gagne `Injury` |
| `domain/events.rs` | le gain de Haine, sans champ de valeur |
| `io/repository/player_repository.rs` | branche de projection : ajout dans `acquired_skills` |
| `io/app_events/player_match_impact_listener.rs` | applique le gain à la conclusion du match, le défait au revert |
| `shared_kernel/app_events/player_match_impact_app_events.rs` | `PlayerInjured` gagne `hatred_skill_uid: Option<String>` — l'uid de la **compétence**, figé à la saisie |

## Le chemin complet

```
Saisie          POST action  →  MatchActionType::Blesse { injury, hatred }
                             →  ActionRecorded          (event store match_report)

Publication     TeamMatchConcluded  →  PlayerInjured { context, injury_type, hatred }
                             →  listener players  →  la compétence désignée est acquise

Dépublication   TeamMatchImpactReverted  →  la Haine est défaite avec le reste

Suppression     ActionDeleted (avant publication)  →  rien n'est parti, rien à défaire
```

**La suppression d'action ne demande aucune compensation** : tant que le rapport
n'est pas publié, aucun événement n'a traversé vers `players`. C'est le
mécanisme existant qui rend cette décision gratuite — c'était l'argument pour ne
pas en inventer un second.

## Trois décisions tranchées

**Le quatrième mode s'appelle `Injury`, pas `Automatic`.** Le coach répond oui
ou non puis choisit parmi trente-huit mots-clefs : c'est le geste le moins
automatique de l'écran. Les trois modes existants nomment la **façon d'obtenir**
— le coach a choisi, le dé a choisi, un commissaire a posé — et la quatrième case
de cette série est « à la suite d'une blessure ». `players` connaît déjà le mot :
`InjuryType`, `PlayerInjuryRecord`, `PlayerInjured`. Le journal affiche
« Blessure », comme « Choisie » traduit `Chosen`.

**`spp_cost` vaut 0, et c'est assumé.** Le champ existe déjà dans
`AcquiredSkillProjection` et décrit un coût réellement payé : zéro est la vérité
pour une Haine. C'est différent d'un `value_delta` à zéro, qui serait un montant
qu'on n'a pas voulu écrire — d'où l'absence de champ de valeur sur l'événement.

**La Haine d'un journalier reste dans le rapport de match.** Un `TempPlayerId`
est un ULID régénéré à chaque match par `init_temp_players_use_case`, aucun
agrégat `players` n'existe pour lui, et aucun mécanisme d'engagement ne le relie
au joueur qu'on recruterait ensuite. Le listener n'émet donc que pour
`ActionPlayer::Regular` ; la Haine d'un temporaire vit dans l'action, s'affiche
au récapitulatif, et sera là le jour où l'engagement existera. C'est la seule
option qui n'invente pas une identité persistante pour les journaliers.

## Trois questions de plus, tranchées

**Les deux listes se partagent sur le roster adverse**, pas sur les joueurs
réellement alignés. `find_roster_positions(opponent_team_id)` rend déjà tous les
postes du roster ; l'union de leurs `keywords` suffit et ne dépend ni de la
feuille de match, ni des journaliers, ni de l'ordre de saisie. Couvrant, et sans
dépendance nouvelle.

**Conséquence sur le libellé** : « Rencontrés dans l'équipe adverse » devient
faux — un poste du roster n'est pas forcément aligné. Le titre du groupe est
donc « Dans le roster adverse ». Un libellé qui promet plus que la donnée ne
tient est un mensonge à retardement.

**Aucune gestion des doublons.** Un joueur peut recevoir deux fois le même
mot-clef ; c'est au coach de ne pas le faire. Cela évite une consultation de
`players` depuis la saisie — le panneau ne connaît pas les Haines déjà acquises —
et une règle de refus dans le domaine. Rien à vérifier, rien à afficher en grisé,
rien à tester.

**Le corpus de production portera `keywords_fr.json`**, et aussi des fichiers
`*_en.json`. Le chargement actuel lit les noms de fichiers **en dur**
(`read_json::<KeywordsFile>(dir, "keywords_fr.json")`), comme pour les onze
autres : la présence de fichiers `_en` ne change rien tant que la carte 395 n'a
pas tranché entre une instance par langue et une instance bilingue. À ne pas
anticiper ici — mais à savoir, pour ne pas coder un choix de langue en passant.

Il reste à prévoir la **garde** : si le corpus ne porte aucun mot-clef, ou aucune
compétence de Haine, la fonctionnalité serait muette. Elle doit échouer
bruyamment, comme le prévoit la carte 388 pour `LOW_COST_LINEMEN`.
