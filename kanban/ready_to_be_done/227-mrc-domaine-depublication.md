# `match_report` — Dépublication dans le domaine

**Priorité : haute**
**Dépend de :** —
**Fichiers :** `src/app/match_report/domain/{value_objects,error,events,match_report_published,match_report_pre_match,match_report_ready_to_publish,match_report_state}.rs`
**Spec :** `docs/specs/match-report-correction/recap/06-domaine.md`

## Objectif

Poser l'arête `Published → ReadyToPublish` dans la machine à états, avec sa
garde métier. Aucune couche IO, aucun port : carte purement domaine, entièrement
couverte par des tests unitaires.

## Conception

### Value objects (`value_objects.rs`)

```rust
pub enum CorrectionEligibility { Eligible, Blocked(CorrectionBlocker) }

pub enum CorrectionBlocker {
    SppAlreadySpent { side: TeamSide },
    PhaseAdvanced   { side: TeamSide },
    EligibilityUnknown,
}
```

`TeamSide` existe déjà dans ce fichier. Le blocker ne porte **aucun nom
d'équipe** : le domaine ignore les chaînes d'affichage (cf. `04-dtos.md`).

### Erreur (`error.rs`)

`DomainError::CorrectionNotAllowed(CorrectionBlocker)` + son bras `Display`.

### Événement (`events.rs`)

```rust
MatchReportUnpublished { unpublished_by: CoachId, unpublished_at: DateTime<Utc> },
```

Plus le bras correspondant dans `type_name()`. Pas de motif — règle 5.

### Méthode domaine

```rust
impl MatchReportPublished {
    pub fn unpublish(&self, unpublished_by: CoachId, eligibility: CorrectionEligibility)
        -> Result<(MatchReportReadyToPublish, MatchReportDomainEvent), DomainError>
}
```

`Blocked(b)` → `Err(CorrectionNotAllowed(b))`. Sinon l'état `ReadyToPublish`
reconstruit depuis `self`, avec `was_published_before: true`.

### Drapeau `was_published_before`

Champ **sur les deux états** `MatchReportPreMatch` et
`MatchReportReadyToPublish`. Le parcours de correction repasse par `PreMatch`
dès la première modification d'action (`into_pre_match()`) : un drapeau porté
par le seul `ReadyToPublish` serait perdu à ce moment-là.

Propagé par `into_pre_match()` **et** par la transition retour vers
`ReadyToPublish`.

### Machine à états

Arête `(Some(Published(p)), MatchReportUnpublished { .. }) → ReadyToPublish`
dans `rehydrate()`.

`rehydrate` étant un `fold`, l'alternance publier/dépublier se traite sans cas
particulier (règle 13) — à démontrer par un test, pas par du code.

## Checklist

- [ ] `CorrectionEligibility` et `CorrectionBlocker` créés
- [ ] `DomainError::CorrectionNotAllowed` + `Display`
- [ ] Événement `MatchReportUnpublished` + `type_name()`
- [ ] `MatchReportPublished::unpublish()`
- [ ] `was_published_before` sur `PreMatch` et `ReadyToPublish`, propagé dans les 2 sens
- [ ] Arête dans `rehydrate()`
- [ ] Test : refus si `SppAlreadySpent`
- [ ] Test : refus si `PhaseAdvanced`
- [ ] Test : refus si `EligibilityUnknown`
- [ ] Test : succès → `ReadyToPublish` avec le drapeau à `true`
- [ ] Test : 3 cycles publier/dépublier successifs rejoués correctement
- [ ] Test : le drapeau survit à `into_pre_match()` puis au retour
- [ ] `make test` passe
- [ ] `make check-arch` passe
