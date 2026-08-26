# Panneau « Points de classement »

**Épic :** E14 · **Ordre :** 3 · **Dépend de :** 418, 420
**Conception :** `docs/specs/modifier-une-competition/onglet-parametres/`
(`03-back.md`, `04-dtos.md`, `05-use-cases.md`)

## Objectif

Modifier le barème d'une saison **en cours**, et rejouer le classement dans la
foulée. C'est le panneau qui porte tout le risque de l'onglet.

## Ce que cette carte ajoute au-delà du panneau

La 418 a donné à `ranking` la capacité de se rejouer. Cette carte pose le
chemin par lequel `competitions` le lui demande.

```rust
// competitions/ports.rs
#[async_trait]
pub trait IRankingRecomputePort: Send + Sync {
    async fn recompute_season(&self, season_id: &str) -> Result<RecomputeReportDto, String>;
}
pub struct RecomputeReportDto { pub matches_replayed: u32, pub teams: u32 }
```

```
src/infrastructure/competitions/ranking_recompute_adapter.rs
    → appelle recompute_season_ranking_use_case
main.rs → instancie l'adapter, l'injecte dans CompetitionsContext
```

**C'est le premier port de `competitions` qui ordonne au lieu de demander** —
les sept autres sont des lectures. Le `CLAUDE.md` range la propagation d'effet
du côté des app events ; on s'en écarte parce que **l'écran doit confirmer**, ce
qu'un événement asynchrone ne permet pas. Un second POST enchaîné par le front
laisserait le barème enregistré sans son recalcul si l'onglet se ferme entre les
deux. Si un second cas de commande synchrone apparaît un jour, la règle mérite
d'être complétée plutôt que contournée une fois de plus.

## Conception

### Le use case

```rust
pub struct UpdateRankingSettingsCommand {
    pub season_id: SeasonId,
    pub ranking_rules: RankingRules,
}
pub struct RankingSettingsOutcome { pub matches_replayed: u32, pub teams: u32 }
```

1. `find_base_info` → le nom de saison
2. `find_rules` → `SeasonNotFound`, **les tiers relus**
3. `save_rules(…)`
4. `recompute.recompute_season(&season_id)` → `RecomputeFailed(String)`

**Le recalcul après l'enregistrement, jamais l'inverse** : il lit le barème par
son propre port, donc il doit lire le nouveau.

**Un recalcul en échec ne défait pas l'enregistrement.** Le barème reste écrit
et l'erreur remonte. C'est acceptable parce que le rejeu est idempotent : le
relancer suffit, là où un rollback rendrait barème et classement incohérents
dans l'autre sens. La variante d'erreur porte ce commentaire — c'est la seule
de l'onglet qui laisse le système à moitié appliqué.

### Le handler

```rust
GET  …/settings/ranking  → get_settings_ranking
POST …/settings/ranking  → post_settings_ranking   (Json)
```

```rust
#[derive(Deserialize)]
pub struct RankingSettingsPayload {
    #[serde(flatten)] pub ranking_rules: RankingRules,
}
```

**JSON et non formulaire** : la cible est un agrégat dont chaque champ est un
nutype qui valide à la désérialisation. Un barème hors bornes est rejeté par
serde avant d'atteindre une ligne de code, et `TiebreakConfig` refuse la liste
vide, les doublons et l'absence de critère actif. **Le handler n'a aucune
validation à écrire.**

**Le recalcul en échec rend `200`, pas `422`** : l'enregistrement demandé a bien
eu lieu, et un `422` rendrait un formulaire déjà sauvegardé.

### Le VM

```rust
pub struct RankingVm {
    pub win_points: u32, pub draw_points: u32, pub lose_points: u32,
    pub offensive: BonusVm, pub defensive: BonusVm, pub aggressive: BonusVm,
    pub tiebreakers: Vec<TiebreakRowVm>,
}
pub struct TiebreakRowVm { pub code: String, pub label: String, pub activated: bool }
pub struct RecomputeVm { pub matches_replayed: u32, pub teams: u32 }
```

`TiebreakRowVm` dépend du catalogue (`ITiebreakCatalogPort::all()`) autant que
du domaine → construit dans `builders.rs`, pas par un `from_domain()`.

Sa construction est une **jointure ordonnée** : l'ordre vient de la
`TiebreakConfig` enregistrée, les libellés du catalogue, et **les critères du
catalogue absents de la configuration s'ajoutent à la fin, désactivés**. Sans
cette dernière règle, un critère ajouté au catalogue serait invisible pour
toutes les compétitions existantes.

### Le template

Bouton avec `hx-indicator` : le POST est le seul long de l'onglet. Au retour, le
pied de panneau annonce le nombre de matchs rejoués.

Le pied de conséquence passe du gris à l'accent quand un champ change — état
d'écran pur, sans aller-retour (maquette).

## Tests

- Unitaires : les tiers relus, l'ordre enregistrement-puis-recalcul, l'échec de
  recalcul qui laisse le barème écrit, la jointure du départage avec un critère
  du catalogue absent de la configuration.
- E2E : **deux matchs joués, victoire passée à 3 points, le classement affiche
  le nouveau total.** C'est le scénario central de toute la fonctionnalité.

## Checklist

- [ ] Le port, l'adapter, l'injection dans `main.rs`
- [ ] Le use case et ses tests
- [ ] Les deux handlers, `require_admin_access`
- [ ] Le VM, `builders.rs`, le template avec `hx-indicator`
- [ ] `make lint && make test && make check-arch`
