# BC `teams` — Repository `find_by_coach_and_space`

**Priorité : haute**
**Dépend de :** rien de nouveau (utilise `team_proj`, déjà à jour — cf. `42-teams-projection.md`)
**Contexte :** `teams` — repository

## Objectif

Ajouter une méthode de lecture permettant de retrouver toutes les équipes
d'un coach dans un space, tous statuts confondus (`PendingEnrollment`,
`Enrolled`, `Rejected`, `Dismissed`), pour alimenter le futur widget
"Mes équipes" (carte 287). Cette carte est un pur ajout de lecture, aucune
migration nécessaire — toutes les colonnes existent déjà sur `team_proj`.

**Spec de référence :** `docs/specs/my-teams/mes-equipes/03-back.md` et `07-integration.md`.

---

## Conception

### DTO de lecture

```rust
// teams/ports.rs
pub struct MyTeamRow {
    pub team_id: String,
    pub team_name: String,
    pub roster_name: String,
    pub logo_url: Option<String>,
    pub status: String,             // "PendingEnrollment" | "Enrolled" | "Rejected" | "Dismissed"
    pub game_phase: Option<String>,
}
```

### Port

```rust
// teams/ports.rs — ajout sur ITeamRepository
async fn find_by_coach_and_space(
    &self,
    coach_id: &str,
    space_id: &str,
) -> Result<Vec<MyTeamRow>, RepositoryError>;
```

### Implémentation

Même patron que `find_enrolled_for_season` (`io/repository/team_repository.rs`),
sans filtre de statut :

```sql
SELECT team_id, team_name, roster_name, logo_url, status, game_phase
FROM team_proj
WHERE coach_id = $1 AND space_id = $2
ORDER BY updated_at DESC
```

---

## Checklist

- [ ] `MyTeamRow` dans `teams/ports.rs`
- [ ] Méthode `find_by_coach_and_space` sur `ITeamRepository`
- [ ] Implémentation SQL dans `TeamRepository` (`io/repository/team_repository.rs`)
- [ ] Test d'intégration repository (vraie PgPool, fixture multi-statuts)
