# Le domaine sait refuser une Haine

**Priorité : haute**
**Dépend de :** 399
**Conception :** `docs/specs/haine/saisie-des-actions/06-domaine.md`
**Fichiers :** `src/app/match_report/domain/value_objects.rs`, `domain/error.rs`,
`domain/match_report_pre_match.rs`,
`src/app/match_report/use_cases/record_action_use_case.rs`

## Objectif

Une blessure peut porter une Haine, et le domaine est seul juge de savoir
laquelle le permet.

```rust
MatchActionType::Blesse {
    injury: InjuryType,
    #[serde(default)]
    hatred: Option<HatredKeyword>,
}

impl InjuryType {
    /// Amoché, Blessure Sérieuse, Séquelle. Une Commotion est trop légère pour
    /// laisser une rancune, une Mort ne laisse personne pour haïr.
    pub fn peut_donner_haine(&self) -> bool
}
```

## `#[serde(default)]`, sinon l'historique devient illisible

Les actions sont persistées **sérialisées en JSON**, dans l'event store comme
dans `match_report_actions.action_json`. Le JSON des blessures déjà écrites ne
porte pas `hatred` : sans `#[serde(default)]`, leur relecture échoue, et le rejeu
d'un rapport ancien avec.

C'est la convention du projet — `events.rs` porte déjà six marqueurs posés pour
cette raison exacte, dont `MatchAction::player_position`.

**Aucune migration** en revanche : la colonne est du JSONB, le champ s'y ajoute
seul.

## La signature de `record_action` change

```rust
pub fn record_action(…) -> Result<(Self, MatchReportDomainEvent), DomainError>
```

Elle rendait un couple. Une action pouvait donc **toujours** être enregistrée ;
ce n'est plus vrai :

```rust
MatchActionType::Blesse { injury, hatred: Some(_) } if !injury.peut_donner_haine()
    => return Err(DomainError::HatredNotAllowedForInjury),
```

**Le use case doit être adapté dans cette carte**, sans quoi le projet ne compile
plus. Il se contente de propager sur `RecordActionError::Domain` — la logique de
refus reste au domaine.

## C'est la source unique de la règle

La constante `PEUT_GAGNER_HAINE` que portera le template (carte 402) n'est qu'un
reflet, chargé de masquer la section. Elle ne décide de rien : un écart entre les
deux se solde par un 422, jamais par une donnée fausse.

## Le value object

```rust
#[nutype(sanitize(trim), validate(not_empty, len_char_max = 40, regex = UID_MOT_CLEF), …)]
pub struct HatredKeyword(String);
```

`UID_MOT_CLEF` vaut `^[A-Z][A-Z0-9_]*$`. Ce sont des identifiants de corpus,
**pas du texte saisi** : `TEXTE_SAISI` ne s'applique pas ici, et l'appliquer
laisserait passer « elfe noir » là où on attend `DARK_ELF`.

Le VO valide **la forme, jamais l'existence** : un uid absent du corpus est
syntaxiquement correct, et c'est le use case qui le refuse (carte 401). Un value
object qui interrogerait un port cesserait d'être un objet du domaine.

## Checklist

- [ ] `HatredKeyword` avec son charset propre
- [ ] `InjuryType::peut_donner_haine()`
- [ ] `Blesse` gagne `hatred` avec `#[serde(default)]`
- [ ] `DomainError::HatredNotAllowedForInjury`
- [ ] `record_action` rend un `Result` ; le use case propage
- [ ] Tests unitaires :
  - [ ] Haine acceptée sur Amoché, Blessure Sérieuse, Séquelle
  - [ ] Haine refusée sur Commotion et sur Mort
  - [ ] **blessure sans Haine acceptée sur les cinq types** — la règle interdit
        la Haine sur une Commotion, pas la Commotion : sans ce test, refuser
        toute blessure légère passerait les deux précédents
  - [ ] les six autres actions inchangées
  - [ ] `peut_donner_haine` sur `Sequel`, quelle que soit la stat
  - [ ] une action historique sans `hatred` se désérialise
  - [ ] `HatredKeyword` : `DARK_ELF` accepté, `dark elf` et vide refusés
- [ ] `make lint`, `make check-arch`, `make test`
