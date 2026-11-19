# Départages — Domaine `competitions` : VOs + erreurs (additif)

**Priorité : haute**
**Dépend de :** — (indépendante de 208/209)
**Contexte :** `src/app/competitions/domain/error.rs`, `src/app/competitions/domain/competition_rules.rs`
**Spec :** `docs/specs/ranking/tiebreakers/competition-rules-form/{04-dtos,06-domaine}.md`

## Objectif

Créer les value objects de configuration des départages et le module d'erreurs domaine
du BC. **Purement additif** : `RankingRules` n'est pas encore touché, le champ
`additionnal_ranking_points` reste en place. La bascule est en carte 211. Commit
intermédiaire sûr.

## Conception (cf. `06-domaine.md`)

### `app/competitions/domain/error.rs` — nouveau

Le BC `competitions` est le seul sans module d'erreurs domaine (`team_creation`,
`players`, `teams`, `match_report` en ont un). Les erreurs nutype sont opaques : un
`predicate` violé ne dit pas lequel, ce qui empêcherait de distinguer les causes de
rejet dans le message 422.

```rust
#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum DomainError {
    #[error("La configuration de départage est vide.")]
    EmptyTiebreakConfig,
    #[error("Au moins un critère de départage doit être actif.")]
    NoActiveTiebreaker,
    #[error("Le critère de départage « {code} » est présent plusieurs fois.")]
    DuplicateTiebreakCode { code: String },
}
```

### `competition_rules.rs` — value objects

```rust
#[nutype(validate(not_empty), derive(Debug, Clone, PartialEq, Eq, Hash,
                                    Serialize, Deserialize, Display, AsRef))]
pub struct TiebreakCode(String);

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TiebreakSetting {
    pub code:      TiebreakCode,
    pub activated: Activated,   // VO existant, ligne 8
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(try_from = "Vec<TiebreakSetting>")]
pub struct TiebreakConfig(Vec<TiebreakSetting>);
```

| Méthode | Rôle |
|---|---|
| `try_new(Vec<TiebreakSetting>) -> Result<Self, DomainError>` | Règles 1, 5, 6 |
| `all_active(Vec<TiebreakCode>) -> Result<Self, DomainError>` | Règle 3 — codes fournis par l'appelant |
| `settings(&self) -> &[TiebreakSetting]` | Lecture ordonnée : l'index **est** la priorité |

**Pas de `default()`** : énumérer les 7 codes ferait connaître le catalogue au domaine
`competitions`, ce que l'architecture refuse (le catalogue appartient à `ranking`).

**Pas de méthode de mutation** : l'enregistrement remplace la configuration en bloc.

### Validation traversée à la désérialisation — impératif

Un `#[derive(Deserialize)]` nu sur un newtype **ne passe pas** par `try_new` : les
invariants seraient contournés par tout payload JSON. D'où le `#[serde(try_from = …)]`
ci-dessus, plus :

```rust
impl TryFrom<Vec<TiebreakSetting>> for TiebreakConfig {
    type Error = DomainError;
    fn try_from(v: Vec<TiebreakSetting>) -> Result<Self, Self::Error> { Self::try_new(v) }
}
```

`Serialize` reste un simple derive : un newtype tuple sérialise comme sa valeur interne,
donc directement en tableau JSON.

## Tests unitaires (cf. `06-domaine.md`)

- `try_new` refuse : liste vide, tous les critères décochés, doublon de code (en nommant
  le code fautif)
- `try_new` accepte une configuration valide et **préserve l'ordre reçu**
- `all_active` : autant de réglages que de codes, tous actifs, dans l'ordre ; refuse une
  liste vide
- Désérialisation : tableau valide → ordre et activation préservés ; tableau sans aucun
  actif → **échoue**
- Sérialisation : tableau `[{code, activated}]`, ordre préservé ; aller-retour stable
- `TiebreakCode` refuse la chaîne vide

## Checklist

- [ ] `domain/error.rs` créé, déclaré dans `domain/mod.rs`
- [ ] `TiebreakCode`, `TiebreakSetting`, `TiebreakConfig` définis ; `Activated` réutilisé
- [ ] `try_from` + `#[serde(try_from)]` en place — vérifié par un test de désérialisation invalide
- [ ] `RankingRules` **non modifié** (la bascule est en 211)
- [ ] Tests unitaires ci-dessus écrits et verts
- [ ] `make test` + `make check-arch` passent
