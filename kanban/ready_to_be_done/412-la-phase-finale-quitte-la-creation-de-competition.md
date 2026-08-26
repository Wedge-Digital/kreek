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

- [ ] `PlayOffsPhase` et son champ dans `CompetitionStructure` retirés
- [ ] Les trois value objects supprimés **après** inventaire de leurs
      consommateurs — Rust, templates, JS inline
- [ ] `play_off_start_date` et `play_off_end_date` retirés **avec leur
      `#[serde(default)]`**
- [ ] Section 2 retirée de `new-competition-phase-3.html`, JS inline compris ;
      sous-titre de l'étape mis à jour
- [ ] `playoffs_label` et `use_playoffs` retirés des trois vues
- [ ] `assets/league_structure.json` nettoyé
- [ ] Tests unitaires :
  - [ ] **une structure enregistrée aujourd'hui, avec `play_offs_phase`, se
        désérialise sans erreur** — le champ est ignoré
  - [ ] une structure sans `scheduled_dates` **échoue toujours** : c'est ce que
        l'attribut orphelin de la carte 334 avait failli casser
  - [ ] sérialisation d'une structure : plus aucune clé de phase finale
- [ ] Test e2e : le magicien de création passe l'étape 3 sans la section 2, et la
      compétition se crée
- [ ] `make lint`, `make check-arch`, `make test`
