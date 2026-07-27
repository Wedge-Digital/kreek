# `teams` — Compensation d'une dépublication

**Priorité : haute**
**Dépend de :** `231-mrc-publisher-app-events.md`
**Fichiers :** `src/app/teams/domain/team.rs`, `src/app/teams/io/repository/team_repository.rs`, `src/app/teams/io/app_events/match_report_unpublished_listener.rs` (nouveau), `src/app/teams/context.rs`
**Spec :** `docs/specs/match-report-correction/recap/06-domaine.md`, `07-integration.md`

## Objectif

Défaire `PostMatchSequenceStarted` sur les 2 équipes : trésorerie, fans, phase de
jeu. **Aucune migration d'événement** — tout est de l'état dérivé.

## Conception

### État dérivé — le cœur de la carte

```rust
struct LastPostMatch {
    match_report_id:       MatchReportId,
    dedicated_fans_before: DedicatedFans,
    treasury_income:       Kpo,
}
// dans Team :
last_post_match: Option<LastPostMatch>,
```

Renseigné dans `apply(PostMatchSequenceStarted)` **avant** que l'état ne soit
écrasé :

```rust
self.last_post_match = Some(LastPostMatch {
    match_report_id:       self.current_match_report_id,  // avant le clear
    dedicated_fans_before: self.dedicated_fans,           // avant l'écrasement
    treasury_income:       *treasury_income,
});
```

Deux informations qu'on croyait perdues sont disponibles à cet instant précis :

- **les fans d'avant le match** — l'événement ne stocke que la valeur
  post-clamp, mais `apply()` voit encore l'ancienne
- **le `match_report_id`** — absent de l'événement, mais
  `current_match_report_id` est encore renseigné : c'est ce même `apply` qui le
  met à `None`

L'état est **rebuildable** depuis les événements existants : aucune migration.

### Événement

```rust
PostMatchSequenceReverted {
    match_report_id: MatchReportId,
    dedicated_fans:  DedicatedFans,  // valeur ABSOLUE restaurée
    treasury_refund: Kpo,
},
```

`dedicated_fans` est une **valeur absolue, pas un delta**. C'est la règle 14 :
`clamp(0, 20)` n'est pas inversible — si `+2` a été écrêté à 20, retrancher 2
donnerait 18 au lieu de 20.

### Méthode domaine

```rust
pub fn revert_post_match_sequence(&self, match_report_id: MatchReportId)
    -> Result<TeamDomainEvent, DomainError>
```

Gardes :
1. `expect_phase(GamePhase::PlayerImprovement)`
2. `last_post_match` renseigné **et** portant ce `match_report_id`

### `apply(PostMatchSequenceReverted)`

```rust
self.dedicated_fans          = *dedicated_fans;
self.treasury.0              = self.treasury.0.saturating_sub(treasury_refund.0);
self.game_phase              = Some(GamePhase::MatchReporting);
self.current_match_report_id = Some(*match_report_id);
self.last_post_match         = None;
```

Restaurer `current_match_report_id` n'est pas cosmétique :
`start_post_match_sequence` exige `MatchReporting`, et la re-publication en
dépend.

`last_post_match = None` **est** le mécanisme d'idempotence : une seconde
compensation ne trouve plus de dernier post-match et refuse.

### Projection

`team_projection` porte `game_phase`, la trésorerie et les fans. La mise à jour
doit se faire **dans la transaction de l'append** (règle ES du CLAUDE.md).

Piège : un événement non traité par la fonction de projection **compile sans
broncher** et laisse l'affichage figé sur les valeurs post-match. D'où un test
d'intégration repository, pas seulement unitaire.

### Listener et câblage

Filtre `MatchReportUnpublished`, traite les 2 équipes du payload sans jamais
croiser home et away. Câblé dans `teams::context::init_listeners`.

## Checklist

- [ ] `LastPostMatch` et le champ `last_post_match` sur `Team`
- [ ] Renseigné dans `apply(PostMatchSequenceStarted)` avant écrasement
- [ ] Événement `PostMatchSequenceReverted` + `type_name()`
- [ ] `revert_post_match_sequence()` avec ses 2 gardes
- [ ] `apply(PostMatchSequenceReverted)` complet, `last_post_match` remis à `None`
- [ ] Projection mise à jour **dans la transaction de l'append**
- [ ] Listener créé et câblé
- [ ] Test : fans écrêtés à 20 → restaurés à 20, pas 18
- [ ] Test : fans plancher à 0 → restaurés à la valeur d'avant
- [ ] Test : trésorerie diminuée du gain
- [ ] Test : phase repassée en `MatchReporting`, `current_match_report_id` restauré
- [ ] Test : refus si la phase a déjà avancé
- [ ] Test : refus si le `match_report_id` ne correspond pas
- [ ] Test : une seconde compensation ne produit rien
- [ ] Test : publier → dépublier → republier converge vers le même état
- [ ] Test d'intégration : la projection reflète l'état compensé
- [ ] `make test` passe
- [ ] `make check-arch` passe
