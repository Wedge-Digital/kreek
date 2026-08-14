# Phase 5 — Use cases : l'écran de réglage des notifications

**Entrée** : `04-dtos.md`, validée.

Deux mutations, donc deux chemins d'écriture — un par mode décidé en phase 2.

## Mutation 1 — l'auto-save de l'admin

`src/app/competitions/use_cases/save_competition_notifications.rs`

```rust
pub struct SaveCompetitionNotificationsCommand {
    pub season_id: SeasonId,
    pub notifications: CompetitionNotifications,
}

#[derive(Debug)]
pub enum SaveCompetitionNotificationsError {
    SeasonNotFound,
    Database(String),
}

impl From<SeasonRepositoryError> for SaveCompetitionNotificationsError { … }

pub async fn execute(
    cmd: SaveCompetitionNotificationsCommand,
    repo: &dyn ISeasonRepository,
) -> Result<(), SaveCompetitionNotificationsError> {
    repo.save_notifications(&cmd.season_id, &cmd.notifications).await?;
    Ok(())
}
```

Même forme que `save_competition_invitations` : commande, enum d'erreurs, `From`
sur l'erreur de repository. Rien d'inventé — c'est la maison.

**Aucune logique métier ici.** Le use case ne décide pas si un réglage est
applicable : l'applicabilité (R5) est une question de lecture, calculée au GET
par le domaine, et un réglage inapplicable **se stocke quand même** (R6). Le
chemin d'écriture n'a donc rien à vérifier.

## Mutation 2 — l'étape 4 du magicien

`save_competition_invitations.rs` gagne un champ :

```rust
pub struct SaveCompetitionInvitationsCommand {
    pub season_id: SeasonId,
    pub invitations: CompetitionInvitations,
    pub notifications: CompetitionNotifications,   // ← nouveau
}
```

et son `execute` écrit les deux colonnes.

### Une seule requête, pas deux — précision sur la phase 4

La phase 4 disait que les deux écritures « peuvent partager une transaction ».
En pratique c'est mieux que ça : **il s'agit de la même ligne**, donc d'un seul
`UPDATE`. La signature du port change plutôt que de gagner une méthode :

```rust
async fn save_invitations(
    &self,
    season_id: &SeasonId,
    invitations: &CompetitionInvitations,
    notifications: &CompetitionNotifications,   // ← nouveau
) -> Result<(), SeasonRepositoryError>;

async fn save_notifications(                     // ← nouveau
    &self,
    season_id: &SeasonId,
    notifications: &CompetitionNotifications,
) -> Result<(), SeasonRepositoryError>;
```

```sql
-- update_invitations.sql — le statut reste, c'est bien le magicien
UPDATE competition_seasons
SET    invitations   = $1::jsonb,
       notifications = $2::jsonb,
       status        = 'invitations_configured'
WHERE  id            = $3
RETURNING id

-- update_notifications.sql — pas de statut, cf. phase 3
UPDATE competition_seasons
SET    notifications = $1::jsonb
WHERE  id            = $2
RETURNING id
```

`save_invitations` n'a qu'un appelant, le changement de signature ne coûte rien.

**L'atomicité n'est pas ce qui compte ici, et autant le dire.** Si les
invitations s'écrivaient sans les notifications, la colonne resterait `NULL`,
donc « tout allumé » (R8) — soit exactement le défaut voulu pour une saison
neuve. Le dégât serait nul. La requête unique est retenue parce qu'elle est plus
simple que deux, pas parce qu'elle sauve quoi que ce soit.

## Aucun événement émis

`save_competition_invitations` n'en émet pas ; les réglages n'en demandent pas
davantage. `envoi/` **lit la colonne au moment du cron** — c'est une consultation
d'état au présent, pas une réaction à un fait passé. Le critère de choix du
CLAUDE.md tranche dans ce sens : port et lecture, pas app event.

Émettre un `NotificationSettingsChanged` n'aurait aujourd'hui aucun abonné, et
un événement sans abonné est une promesse d'API que personne ne tient.

## Erreurs

| Erreur | Cause | Réponse HTTP |
|---|---|---|
| `SeasonNotFound` | la saison n'existe pas | `404` |
| `Database` | l'écriture a échoué | `500` |

Le cas « saison d'un autre espace » ne figure pas dans cette liste : il est
intercepté **avant le handler** par le middleware de cloisonnement, via
`SeasonSpaceOwnership` (phase 3). Le use case n'a pas à le connaître.

## Règles métier

Aucune n'apparaît à cette phase. Les deux qui la contraignent existent déjà :
R5 ne concerne que la lecture, R6 impose que l'écriture ne filtre rien.

## Ce que cette phase laisse à la suivante

`applicability()` — la seule logique métier de cet écran, et le seul endroit où
une décision se prend.
