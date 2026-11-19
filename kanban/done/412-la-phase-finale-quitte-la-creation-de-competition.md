# La phase finale quitte la création de compétition

**Priorité : moyenne** — un réglage qui ne sert pas, et qu'il faut pourtant remplir
**Dépend de :** rien
**Fichiers :** `src/app/competitions/domain/competition_structure.rs`,
`io/web/templates/new-competition-phase-3.html`,
`io/web/new_competition_phase_5.rs`, `io/web/competition_widget.rs`,
`io/web/admin/summary_tab.rs`, `assets/league_structure.json`

## Objectif

La section **« 2 — Phase finale (play-offs) »** disparaît de l'étape 3 du
magicien, et le modèle cesse de la porter. Rien dans kreek ne s'en sert : aucun
appariement de phase finale n'est généré, aucun classement n'en tient compte.

Portée retenue : **l'écran et le modèle**. Les structures déjà enregistrées ne
sont pas migrées.

## Ce qui part

**Le domaine** (`competition_structure.rs`) :

```rust
pub struct CompetitionStructure {
    pub ranking_group: RankingGroupConfig,
    pub play_offs_phase: PlayOffsPhase,     // ← part
    pub schedule: ScheduleConfig,
}

pub struct PlayOffsPhase {                   // ← part en entier
    pub use_playoffs_phase: UsePlayoffsPhase,
    pub qualified_team_per_pool: QualifiedTeamPerPool,
    pub final_phase_match_for_third_place: FinalPhaseMatchForThirdPlace,
}
```

Et les trois value objects qui n'auront plus de consommateur —
`UsePlayoffsPhase`, `QualifiedTeamPerPool`, `FinalPhaseMatchForThirdPlace`. **À
vérifier avant de supprimer**, comme l'exige la règle 4 du `CLAUDE.md` : lister
les consommateurs, y compris dans les templates et le JS inline.

**Deux dates du calendrier** partent avec, dans `ScheduleConfig` :

```rust
#[serde(default)] pub play_off_start_date: DateString,
#[serde(default)] pub play_off_end_date: DateString,
```

Elles bornent une phase qui n'existera plus.

**L'écran** : la section 2 de `new-competition-phase-3.html` — le bloc de choix
`#playoff-mode-btns`, le panneau `#playoff-config`, les deux champs de date, et
les trois endroits du JS inline qui les lisent ou les écrivent (lignes ~262,
~331, ~395). Le sous-titre de l'étape devient « Poules & calendrier ».

**Les libellés dérivés**, dans trois vues : `new_competition_phase_5.rs`
(`playoffs_label`), `competition_widget.rs` (`use_playoffs`),
`admin/summary_tab.rs` (`playoffs_label`).

**Le fichier d'exemple** `assets/league_structure.json` : le bloc
`play_offs_phase` et les deux dates.

## Les 1114 structures déjà en base

Toutes les structures enregistrées portent `play_offs_phase` dans leur JSONB —
**1114 lignes**, soit la totalité.

Elles n'ont **pas besoin d'être migrées** : serde ignore les champs inconnus par
défaut, et le projet ne pose `deny_unknown_fields` nulle part — vérifié. Le champ
résiduel sera lu et jeté.

**Mais c'est à vérifier, pas à supposer** : un test de désérialisation sur une
structure complète d'aujourd'hui, après retrait du champ, appartient à cette
carte. Si `deny_unknown_fields` apparaissait un jour, ces 1114 lignes cesseraient
de charger d'un coup.

## Le piège que la carte 334 a rencontré, sur ce même fichier

En retirant deux champs de `ScheduleConfig`, elle a laissé un `#[serde(default)]`
**orphelin**, qui s'est recollé au champ suivant — `scheduled_dates`, obligatoire.
Une structure sans journées se serait alors désérialisée au lieu d'échouer, et
personne ne l'aurait vu.

Ici, deux champs de la même struct partent. **Retirer chaque attribut avec son
champ**, et relire le diff plutôt que le fichier final : c'est à la relecture du
diff que l'orphelin s'était vu.

## Ce que la carte ne fait pas

- **Aucune migration de données.** Le champ reste dans le JSONB des structures
  existantes, inerte.
- **Rien sur les appariements ni le classement** : ils n'ont jamais connu les
  phases finales.
- **Rien sur les prolongations et tirs au but** du règlement (p. 55), qui
  concernent le déroulement d'un match à élimination directe — autre sujet, pas
  encore couvert non plus.

## Checklist

- [x] `PlayOffsPhase` et son champ dans `CompetitionStructure` retirés
- [x] Les trois value objects supprimés **après** inventaire de leurs
      consommateurs — Rust, templates, JS inline
- [x] `play_off_start_date` et `play_off_end_date` retirés **avec leur
      `#[serde(default)]`**
- [x] Section 2 retirée de `new-competition-phase-3.html`, JS inline compris ;
      sous-titre de l'étape mis à jour **et sections renumérotées**
- [x] `playoffs_label` et `use_playoffs` retirés des trois vues
- [x] `assets/league_structure.json` nettoyé
- [x] Les trois tests unitaires
- [x] Test e2e : l'étape 3 sans la section, et la structure écrite
- [x] `make lint`, `make check-arch`, `make test`

## Douze fichiers, là où la carte en nommait six

L'inventaire exhaustif qu'exige la règle 4 en a trouvé quatre de plus :

| Fichier | Ce qu'il portait |
|---|---|
| `competition_notifications.rs` | un littéral JSON, dans un test |
| `test_season_structure_pruning.rs` | des fixtures |
| `update_pools_settings_use_case.rs` | **le code de la carte 423**, qui préservait délibérément `play_offs_phase`, deux tests qui l'affirmaient, et un commentaire qui l'expliquait |
| trois gabarits d'affichage | `summary.html`, `competition-widget-detail.html`, `new-competition-phase-5.html` |

Le commentaire de la 423 a été **réécrit**, pas amputé : il disait « `schedule` et
`play_offs_phase` sont conservés », et il ne reste qu'un champ à préserver.

Le **JS en ligne comptait quinze points**, pas trois. Deux d'entre eux —
`document.getElementById('playoff-start-date').value = …` — repeuplaient les
champs au retour sur l'étape : laissés en place, ils auraient planté sur un
élément absent.

## L'orphelin de la carte 334, évité et gardé

`ScheduleConfig` compte désormais **trois** `#[serde(default)]` pour trois champs
optionnels, et aucun sur `scheduled_dates`. Vérifié à la relecture du diff, comme
la carte le demande — c'est là que l'orphelin s'était vu la première fois.

Et vérifié par mutation : déplacer l'attribut sur `scheduled_dates` fait rougir
`une_structure_sans_journees_echoue_toujours`.

## La renumérotation, que la carte ne demandait pas

Retirer la section 2 aurait laissé « 1, 3, 4 » — un saut que le lecteur attribue
à un défaut d'affichage plutôt qu'à un choix. Les sections sont désormais 1, 2, 3.

## `assets/league_structure.json` redevient du JSON valide

Il portait une virgule finale — il n'était pas analysable. Le bloc qui la
contenait est parti avec la phase finale. Le fichier n'est lu par personne : c'est
de la documentation, et elle était fausse.

## Falsification

| Mutation | Constaté |
|---|---|
| L'attribut `#[serde(default)]` déplacé sur `scheduled_dates` | `une_structure_sans_journees_echoue_toujours` rouge |
| `deny_unknown_fields` ajouté | 2 tests rouges, dont celui des structures en base |
| La section 2 remise à l'écran | le test e2e d'écran rouge |
| **Le JS renvoie `play_offs_phase`** | **rien ne rougit** — cf. ci-dessous |

## Ce que les tests ne peuvent pas voir, et qu'il faut savoir

Un JS resté en arrière, qui poserait encore `play_offs_phase` dans sa charge
utile, **passerait inaperçu** : le serveur jette les champs inconnus, et la
structure enregistrée resterait propre. Mesuré en falsifiant.

C'est la contrepartie exacte de la tolérance dont cette carte dépend pour lire
les 3330 structures anciennes. Elle vaut aussi pour le front, et aucun contrôle
ne s'y oppose. La docstring du fichier e2e le dit, plutôt que de laisser croire
au lecteur que le test couvre ce cas.

## Un e2e écrit puis retiré

Une troisième assertion vérifiait qu'une saison d'avant le retrait reste
lisible, en interrogeant sa page d'administration. Elle se contentait d'un
`200` — or cette page rend `200` que la structure ait été lue ou non : sous
`deny_unknown_fields`, elle répondait toujours `200`, la structure ayant
simplement **disparu de son contenu**.

Le test passait dans les deux cas. Il est retiré, avec son motif écrit dans le
fichier : cette assertion relève de serde, et le test unitaire la tient — en
rougissant sous la même mutation.

## Ce qui reste, et pourquoi

3330 structures portent encore `play_offs_phase` dans leur JSONB, inerte. La
carte ne les migre pas, et ce choix est tenu. Une migration de nettoyage serait
une autre carte.
