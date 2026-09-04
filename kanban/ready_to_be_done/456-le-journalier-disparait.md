# Le journalier disparaît

**Épic :** E15 — Recruter un journalier
**Ordre :** 3 · **Dépend de :** 455
**Conception :** `docs/specs/embaucher-un-journalier/` (`00-conception.md`
décisions 13 et 15, `ecran-de-recrutement/07-integration.md`)

## Objectif

Qu'un journalier non recruté quitte l'effectif à la fin de la phase de
recrutement — et qu'un journalier d'un rapport annulé n'y ait jamais laissé de
trace.

Sans cette carte, la 455 produit des journaliers qui s'accumulent.

## Le manque qu'elle comble

`TeamsAppEvent` ne compte que **deux** variantes — `PlayerRecruited` et
`PlayerDismissed` — et **aucune ne parle de phase**.

Or `players` doit faire le ménage lui-même, en écoutant la sortie de
`Recruitment` : il lui faut donc un app event à écouter, qui n'existe pas.

Le **domain event** existe déjà — `RecruitmentPhaseValidated` (`team.rs:159`),
qui fait passer l'équipe en `Dismissals`. Il lui manque son bras dans le
`to_app_event()` du publisher.

```rust
TeamsAppEvent::RecruitmentPhaseValidated { event_id, team_id, space_id }
```

**Cet événement servira au-delà de cette fonctionnalité** : tout BC qui voudra
réagir à la fin d'une phase de recrutement l'aura.

## Conception

### 1. La perte à la sortie de phase

```rust
// players/io/app_events/recruitment_phase_listener.rs
// Passe en Dismissed tous les Journeyman restants de cette équipe.
```

Il s'exécute **après** le lot d'événements de la validation de phase, qui
contient déjà les basculements en `Active` des journaliers recrutés. L'ordre est
garanti par la structure : on ne peut pas perdre un journalier qu'on vient de
recruter.

### 2. L'événement de perte n'est pas un renvoi

```rust
PlayerJourneymanLost { player_id }
```

**Et non `PlayerDismissed`.** Un journalier perdu n'a pas été renvoyé par une
décision du coach — il n'a simplement pas été retenu.

Les confondre ferait apparaître ces joueurs dans l'historique des renvois, où
ils raconteraient une décision qui n'a jamais été prise.

Le `membership` devient `Dismissed` dans les deux cas ; c'est l'**événement**
qui diffère, donc l'histoire.

### 3. L'annulation supprime, elle ne marque pas

```rust
// players/io/app_events/match_report_cancelled_listener.rs
```

Un journalier d'un rapport annulé **n'a jamais joué**. Le garder en `Dismissed`
polluerait l'effectif d'une trace de rien.

**L'événement porte les `player_id`**, plutôt que de laisser `players` retrouver
les `Journeyman` de l'équipe : la seconde voie supprimerait aussi ceux d'un
rapport antérieur non encore traité — cas rare, mais destructeur.

`MatchReportCancelled` doit donc gagner cette liste, et `players` doit se mettre
à l'écouter — il ne le fait pas aujourd'hui.

### 4. La dépublication ne touche à rien

**Une embauche survit à une dépublication** : le coach a payé, la décision lui
appartient.

Ce qui se défait, ce sont les SPP et les blessures — que
`TeamMatchImpactReverted` gère déjà, et qui s'appliquent au journalier comme aux
autres puisqu'il est un joueur ordinaire.

Un journalier **non encore recruté** sur un rapport dépublié reste recrutable,
avec sa valeur recalculée : la correction a simplement changé ce qu'il vaut.

**Rien à écrire pour ce cas** — c'est une non-action, à vérifier par un test.

## Tests

| Test | Ce qu'il prouve |
|---|---|
| `les_journaliers_restants_sont_perdus_a_la_sortie_de_phase` | décision 13 |
| **`un_journalier_recrute_survit_a_la_sortie_de_phase`** | l'ordre du lot |
| `un_journalier_perdu_n_est_pas_un_renvoye` | deux événements distincts |
| `l_annulation_supprime_les_journaliers` | décision 15 |
| `l_annulation_ne_touche_pas_ceux_d_un_autre_rapport` | pourquoi l'événement porte les identifiants |
| `la_depublication_ne_change_pas_le_membership` | la non-action |

`un_journalier_recrute_survit_a_la_sortie_de_phase` est le test qui compte : il
échoue si quelqu'un déplace un jour le ménage **avant** le lot de validation, ce
qui compilerait parfaitement et perdrait un joueur qu'on vient de payer.

## Checklist

- [ ] `TeamsAppEvent::RecruitmentPhaseValidated` et son bras de publisher
- [ ] Le listener de perte, `PlayerJourneymanLost`
- [ ] `MatchReportCancelled` porte les `player_id`, et `players` l'écoute
- [ ] Les six tests
- [ ] `make lint && make test && make check-arch`
