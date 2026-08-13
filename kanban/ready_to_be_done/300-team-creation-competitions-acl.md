# `team_creation` lit directement les repositories de `competitions`

**Priorité : moyenne**
**Dépend de :** —
**Contexte :** `team_creation` — ACL vers `competitions`

## Problème

Cinq accès directs au contexte du BC `competitions` depuis `team_creation` :

| Fichier | Ligne | Accès |
|---|---|---|
| `io/web/finalize_team.rs` | 83 | `state.competitions.season_repository.find_invitations()` |
| `io/web/finalize_team.rs` | 208 | idem |
| `io/web/build_team/submit_team.rs` | 31 | idem |
| `io/web/build_team/display_page.rs` | 54 | `state.competitions.competition_repository.find_base_info()` |
| `io/web/build_team/display_page.rs` | 59 | `state.competitions.season_repository.find_base_info()` |

Violation de la souveraineté des données entre BCs : `team_creation` lit les
tables de `competitions` sans passer par un port.

**Restées invisibles jusqu'ici** parce que l'axe 3 de `check-arch.sh` était
doublement aveugle — `find -printf` vidait la liste des BCs sur macOS, et les
chaînages coupés par `rustfmt` échappaient au grep ligne à ligne. La carte 297
répare les deux et les découvre d'un coup. Elles sont tolérées par la ligne de
base de l'axe 3 en attendant cette carte, et **doivent en sortir** une fois
faite.

## Ce que `team_creation` a réellement besoin de savoir

Deux questions seulement, malgré cinq appels :

1. **« Cette saison accepte-t-elle les inscriptions sans validation ? »**
   — trois appels, tous réduits à `!invitations.requires_validation`.
2. **« Comment s'appellent cette compétition et cette saison ? »**
   — deux appels, pour un affichage.

Aucun des cinq n'a besoin des agrégats complets : le port peut donc rester
étroit.

## Action

Port dans `team_creation/ports.rs` — nom et découpage à discuter en démarrant
la carte, mais l'esprit :

```rust
pub struct SeasonEnrollmentDto { pub auto_accepts_enrollment: bool }
pub struct CompetitionNamesDto { pub competition_name: String, pub season_name: String }

#[async_trait]
pub trait ICompetitionInfoPort: Send + Sync {
    async fn season_accepts_enrollment(&self, season_id: &SeasonId) -> Option<bool>;
    async fn find_names(&self, competition_id: &..., season_id: &...) -> Option<CompetitionNamesDto>;
}
```

Adapter dans `src/infrastructure/team_creation/`, instancié dans `main.rs` et
injecté dans `TeamCreationContext` — même patron que le `reference_data_adapter`
déjà en place pour ce BC.

**Point à trancher au démarrage** : un seul port à deux méthodes, ou deux ports
(inscription / libellés) ? Les deux questions n'ont ni le même consommateur ni
la même nature — l'une décide, l'autre affiche.

## Checklist

- [ ] Découpage du port tranché (un port ou deux)
- [ ] Port + DTOs dans `team_creation/ports.rs`
- [ ] Adapter dans `src/infrastructure/team_creation/`
- [ ] Injection dans `TeamCreationContext` via `main.rs`
- [ ] Les cinq appels passent par le port, plus aucun `state.competitions`
- [ ] Entrée `team_creation` retirée de `AXE3_BASELINE_REGEX` dans `check-arch.sh`
- [ ] `make check-arch` passe sans elle
- [ ] Tests e2e impactés au vert (`team_creation` est traversé par presque toute la suite)
