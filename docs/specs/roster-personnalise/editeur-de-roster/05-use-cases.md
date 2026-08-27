# Éditeur de roster · Phase 5 : use cases

**Phase 4** : `04-dtos.md`

## Trois mutations, trois use cases

```
references/use_cases/
├── create_custom_roster_use_case.rs
├── update_custom_roster_use_case.rs
└── delete_custom_roster_use_case.rs
```

**Le dossier n'existe pas** : `references` n'a jamais eu de couche applicative.
Il servait des données lues au démarrage, sans une seule écriture. Cette
fonctionnalité lui en donne une.

## Le tier est une liste fermée — et ce n'est pas `TierName`

Tranché en phase 4. Mais `TierName` existe déjà et **désigne autre chose** :

```rust
// shared_kernel/bloodbowl/tier.rs
#[nutype(sanitize(trim), validate(not_empty, len_char_max = 50, regex = TEXTE_SAISI))]
pub struct TierName(String);
```

C'est le nom qu'un organisateur donne à une **catégorie de sa compétition** —
« Débutants », « Confirmés ». Texte libre, et c'est juste : il nomme ce qu'il
veut.

Le tier d'un **roster** est autre chose : un classement de puissance défini par
le règlement, que le corpus écrit `"Tier 1"`, `"Tier 2"`, `"Tier 3"`.

```rust
// shared_kernel/bloodbowl/roster.rs
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RosterTier { One, Two, Three }
```

Sérialisé `"Tier 1"` … `"Tier 3"` pour rester la forme du corpus.

**Les confondre coûterait cher** : un roster prendrait un tier « Débutants » que
rien ne saurait comparer, et le classement de puissance cesserait d'être une
échelle.

## Le geste commun

Les trois partagent la même ouverture, et elle n'est pas décorative :

```
1. l'appelant est-il admin de cet espace ?        → Forbidden
2. le roster existe-t-il, et dans CET espace ?    → NotFound
3. combien d'équipes le jouent ?                  → InUse { teams }
```

**`NotFound` et non `Forbidden` pour un roster d'un autre espace.** Un
`Forbidden` confirmerait son existence à qui énumère. C'est la règle que
`space_scope` applique déjà — « `404` et non `403` pour une ressource
étrangère : un `403` confirmerait son existence à qui l'énumère ».

L'étape 3 ne concerne pas la création.

## 1 · `create_custom_roster_use_case`

```rust
pub async fn execute(
    cmd: CreateCustomRosterCommand,
    repo: &dyn IReferenceWriteRepository,
    admin: &dyn IReferencesSpaceAdminPort,
    id_service: &dyn IdService,
) -> Result<RosterUid, CustomRosterError>
```

1. `admin.is_space_admin(...)` → `Forbidden`
2. **valider le roster** — les règles de la phase 6
3. engendrer les uid : `CUSTOM_<sulid>` pour le roster, `<uid>__<SULID>` par poste
4. résoudre les limites croisées : les **index** de la commande deviennent les
   uid qu'on vient d'engendrer
5. convertir en `Team`
6. `repo.save_custom_roster(space_id, &team, created_by)`

**Les uid des postes viennent d'un identifiant engendré, pas d'un slug du nom.**
Un slug casse au renommage, et deux postes homonymes produiraient le même uid.
Le corpus écrit `DEMO_GRANIT__PIETAILLE` parce qu'il est écrit à la main une
fois ; ici les noms bougent.

**Aucun événement.** Personne n'a à réagir à un roster qui apparaît : il se
résout par uid au moment d'en avoir besoin.

## 2 · `update_custom_roster_use_case`

```rust
pub async fn execute(
    cmd: UpdateCustomRosterCommand,   // = la commande de création + le uid
    repo: &dyn IReferenceWriteRepository,
    admin: &dyn IReferencesSpaceAdminPort,
    usage: &dyn IRosterUsagePort,
) -> Result<(), CustomRosterError>
```

1. accès, existence, appartenance à l'espace
2. `usage.count_teams_using(uid)` → **si > 0, `InUse { teams }`**
3. valider
4. **conserver le uid du roster**, et ceux des postes qui subsistent
5. `repo.save_custom_roster(...)`

### Le verrou se re-vérifie ici, pas seulement à l'écran

L'écran affiche « Modifier » sur un compteur à zéro. Entre l'affichage et
l'enregistrement, une équipe peut naître. **Le use case recompte** : l'écran
avertit, le serveur tranche.

### Les uid des postes survivent quand le poste survit

Un poste modifié garde son uid ; un poste ajouté en reçoit un neuf ; un poste
retiré emporte le sien.

C'est sans conséquence tant que le roster n'est pas utilisé — et il ne l'est
pas, sinon on n'en serait pas là. **Mais ça le sera le jour où l'on autorisera
la modification d'un roster joué**, et changer les uid sous les pieds des
joueurs existants les détacherait de leur poste. Le faire correctement
maintenant coûte le même effort.

## 3 · `delete_custom_roster_use_case`

```rust
pub async fn execute(
    uid: &RosterUid,
    space_id: &SpaceId,
    repo: &dyn IReferenceWriteRepository,
    admin: &dyn IReferencesSpaceAdminPort,
    usage: &dyn IRosterUsagePort,
    bus: &EventBus,
) -> Result<(), CustomRosterError>
```

1. accès, existence, appartenance
2. `count_teams_using(uid)` → si > 0, `InUse { teams }`
3. `repo.delete_custom_roster(uid)`
4. `emettre(bus, ReferencesDomainEvent::CustomRosterDeleted { uid, space_id })`

**`emettre()` et non `.send()`** — c'est la règle de l'axe 12 de `check-arch`.
Le helper est le seul à voir l'enveloppe produite, et `to_enveloppe()` engendre
un identifiant : une ligne de journal écrite à la main au-dessus d'un `send`
reprendrait celui de l'enveloppe reçue et corrélerait n'importe quoi.

**Les tiers ne bloquent pas.** Un tier de compétition qui cite ce roster est mis
à jour **après coup**, par le listener de `competitions` (phase 3). C'est la
doctrine : propagation d'un effet → app event.

### Ce que `references` doit acquérir pour émettre

Ce BC n'a **jamais rien publié**. Il lui faut, dans cet ordre :

| Quoi | Où |
|---|---|
| `ReferencesDomainEvent` | `references/domain/domain_event.rs` |
| Un bus interne dans son contexte | `references/context.rs` — qui ne porte aujourd'hui qu'un `repository` |
| Un publisher | `references/io/app_events/app_event_publisher.rs` |
| `ReferencesAppEvent` et son `to_app_event()` | `shared_kernel/app_events/` |
| Le listener côté `competitions` | `competitions/io/app_events/custom_roster_deleted_listener.rs` |

C'est la moitié du travail de cette carte, et rien de tout ça n'existe.

## Le cache n'apparaît pas dans les use cases

Phase 3 : toute écriture écrit en base **puis** rafraîchit la carte en mémoire.
Ce second geste est un **détail d'implémentation du dépôt**, pas une étape
d'orchestration.

```rust
// IReferenceWriteRepository — ce que le use case voit
async fn save_custom_roster(&self, space_id: &SpaceId, team: &Team, by: &CoachId)
    -> Result<(), RepositoryError>;
async fn delete_custom_roster(&self, uid: &RosterUid) -> Result<(), RepositoryError>;
```

L'implémentation écrit, puis rafraîchit, et journalise si le rafraîchissement
échoue. **Le use case n'a pas à savoir qu'un cache existe** — le lui faire
piloter reviendrait à mettre la persistance dans l'orchestration.

## Les erreurs

```rust
pub enum CustomRosterError {
    Forbidden,
    NotFound,
    /// Le compte est dans l'erreur : l'écran dit « 3 équipes le jouent »,
    /// pas « impossible ».
    InUse { teams: u32 },
    Invalid(DomainError),
    UsageUnavailable(String),
    Repository(String),
}
```

**`InUse` porte le nombre**, parce que l'écran doit nommer la cause et pas
seulement l'interdit (phase 2). Une erreur qui dit « non » envoie chercher.

**`UsageUnavailable` est distincte de `Repository`.** Si le port vers `teams`
échoue, on ne sait pas si le roster est utilisé — et **on refuse**. Traiter
l'indisponibilité comme un zéro laisserait supprimer un roster joué par cent
équipes parce qu'une requête a échoué. Le doute ferme la porte.

## Ce que les use cases ne font pas

- **Aucune transaction.** Une seule écriture par mutation ; le rafraîchissement
  du cache ne participe à aucune transaction Postgres et ne le peut pas.
- **Aucun nettoyage des brouillons.** Leur liste de rosters est une copie figée,
  un uid mort y est sans effet (phase 3).
- **Aucune validation de règle spéciale inventée** : le sélecteur ne propose que
  les existantes, et le serveur refuse le reste par le contrôle de cohérence.

## Règles métier

**Toutes tranchées.** Récapitulées pour la phase 6 :

| | Règle |
|---|---|
| 1 | Seul un admin d'espace crée, modifie ou supprime |
| 2 | Un roster d'un autre espace est **introuvable**, pas interdit |
| 3 | Un roster joué par au moins une équipe ne se modifie ni ne se supprime |
| 4 | Le compte d'usage est re-vérifié au moment de l'écriture, jamais fait confiance à l'écran |
| 5 | Un port d'usage indisponible **refuse** l'opération |
| 6 | Le tier appartient à une liste fermée de trois valeurs |
| 7 | Les uid sont engendrés, jamais dérivés d'un nom |
| 8 | Un poste qui subsiste garde son uid |
| 9 | La suppression émet un domain event ; les tiers suivent, ils ne bloquent pas |
