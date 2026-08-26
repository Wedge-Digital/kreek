# Onglet Trésorerie · Phase 3 : architecture back

**Phase 2** : `02-front.md`

## Le grand livre n'a jamais été lu

`teams__treasury_ledger` est alimenté depuis l'origine, **dans la transaction de
l'append** de l'événement qui produit le mouvement
(`team_repository.rs:310`, `ON CONFLICT (team_id, event_version) DO NOTHING` —
rejouer un événement ne duplique pas sa ligne).

Mais **aucun code de production ne le lit**. Les seules lectures sont dans les
tests du dépôt. Tout le chemin de lecture est donc à écrire — c'est la bonne
nouvelle de cette phase : rien n'est à défaire.

```rust
// ITeamRepository — la seule méthode de lecture nécessaire
async fn list_treasury_movements(&self, team_id: &TeamId)
    -> Result<Vec<TreasuryMovementRow>, RepositoryError>;
```

```rust
/// DTO de lecture — primitives assumées, aucun invariant à protéger.
pub struct TreasuryMovementRow {
    pub event_version: i64,
    pub direction: String,        // "Credit" | "Debit"
    pub amount_kpo: i32,
    pub reason: String,           // les 8 motifs de MovementReason
    pub balance_after_kpo: i32,
    pub occurred_at: DateTime<Utc>,
    pub payload: serde_json::Value,   // l'événement joint, cf. ci-dessous
}
```

**Une seule requête, avec la jointure d'origine :**

```sql
SELECT l.event_version, l.direction, l.amount_kpo, l.reason,
       l.balance_after_kpo, l.occurred_at, e.payload
FROM   teams__treasury_ledger l
LEFT   JOIN team_event_store e
       ON e.team_id = l.team_id AND e.version = l.event_version
WHERE  l.team_id = $1
ORDER  BY l.event_version
```

`event_version` **est** la version de l'agrégat : la jointure rend l'événement
exact qui a produit le mouvement, donc son détail, **sans changer le schéma**.
`LEFT JOIN` et non `JOIN` : une ligne dont l'événement manquerait doit
s'afficher sans détail plutôt que disparaître du relevé — un relevé à trou est
pire qu'un relevé sommaire.

`ORDER BY event_version` et non `occurred_at` : deux mouvements du même
traitement partagent l'horodatage à la milliseconde près, et c'est la version
qui porte l'ordre réel — celui dans lequel les soldes s'enchaînent.

## Le détail, source par source

Chaque motif tire son détail d'un endroit différent. Le tableau est le cœur de
cette phase :

| Motif | Détail affiché | Source | Coût |
|---|---|---|---|
| `InitialEndowment` | « Création de l'équipe » | rien à lire | — |
| `MatchIncome` | « Victoire » / « Défaite » / « Match nul » | `PostMatchSequenceStarted.result` | **gratuit** |
| `MatchIncomeReverted` | « Rapport de match corrigé » | libellé fixe | — |
| `PlayerRecruitment` | « Gwenn, Passeuse — n° 7 » | `ISquadPort`, **déjà utilisé** par `team_value_service` | **gratuit** |
| `StaffPurchase` | « Apothicaire × 1 » | `StaffBought.staff_type` et `quantity` | **gratuit** |
| `InducementPurchase` | — | voir « ce qu'on abandonne » | — |
| `InducementRefunded` | « Rendus avec l'annulation du rapport » | libellé fixe | — |
| `CostlyMistake` | l'incident et le jet | `CostlyMistakesApplied` (épic E13) | **gratuit** |

**Cinq des huit motifs tirent leur détail du propre flux d'événements de
`teams`**, et le sixième d'un port qu'il possède déjà. Le « détail » validé en
phase 1 coûte donc beaucoup moins que ce que cette phase-là laissait croire.

## Correction de la phase 1 : le port ne va pas vers `match_report`

La phase 1 annonçait un port vers `match_report` pour l'adversaire, le score et
la journée. **C'est la mauvaise cible.**

`match_report` ne connaît pas le score : aucune de ses tables n'en porte. Le
score, le nom de la journée et les noms des deux équipes vivent **ensemble, sur
une seule ligne**, dans `competition_match_display_proj` — la projection
d'affichage de `competitions`, indexée notamment par `match_report_id` :

```
J1 | Granitiers | Zéphyriens | 2 | 0 | completed
```

Un port vers `competitions` remplace donc le port vers `match_report`, et rend
en **une lecture** ce qui en aurait demandé trois.

```rust
// teams/ports.rs
#[async_trait]
pub trait IMatchContextPort: Send + Sync {
    /// Le contexte d'un match, pour légender une ligne de trésorerie.
    /// `None` si le match n'a pas de ligne d'affichage — cf. la réserve
    /// ci-dessous.
    async fn find_match_context(&self, match_report_id: &str) -> Option<MatchContextDto>;
}

pub struct MatchContextDto {
    pub round_name: String,        // « J1 »
    pub home_team_name: String,
    pub away_team_name: String,
    pub home_score: Option<u8>,    // absent tant que le match est en cours
    pub away_score: Option<u8>,
}
```

L'adapter vit dans `src/infrastructure/teams/match_context_adapter.rs`, aux
côtés des cinq qui y sont déjà. **Le BC `teams` n'importe jamais
`competitions`** — seul l'adapter le fait.

Les `match_report_id` viennent des charges utiles déjà jointes :
`InducementsPaid`, `InducementsRefunded` et `PostMatchSequenceReverted` les
portent. `PostMatchSequenceStarted`, lui, **n'en porte pas** — le résultat y
suffit, et c'est pour cela que la ligne de recette affiche « Victoire » sans
score.

**Le regroupement par journée en découle** : il n'est possible que pour les
lignes dont l'événement porte un `match_report_id`. Les autres se rattachent au
groupe ouvert par la dernière ligne qui en portait un.

## Une dépendance à nommer : la carte 427

`find_match_context` interroge `competition_match_display_proj`. Or **un rapport
de match créé manuellement n'y a aucune ligne** tant qu'il n'est pas publié —
c'est exactement le défaut de la carte 427, dont le listener de confirmation
abandonne en silence sur un `pairing_id` absent.

Conséquence directe : pour un match démarré hors calendrier, le port rendra
`None`, et les lignes de coups de pouce n'auront **ni journée, ni adversaire**.

Ce n'est pas un défaut de cette fonctionnalité, et il ne la bloque pas — le
gabarit se replie proprement. Mais **la 427 livrée, le relevé se complète tout
seul**, sans qu'on y revienne. C'est un argument de plus pour la prendre avant.

## Ce qu'on abandonne, et pourquoi

**Les noms des coups de pouce.** La maquette affiche « Apothicaire itinérant ·
Chef cuisinier ». L'information existe — `match_report_proj.home_inducements`
porte `[{"uid": "…", "qty": 1, "unit_cost": 60}]` — mais l'atteindre demande un
**second port**, vers `match_report`, puis une **troisième résolution**, vers le
corpus de références, pour changer les uid en libellés. Trois sources pour un
motif sur huit.

La ligne dira donc « Coups de pouce » et son montant. Ce qui a été acheté se lit
dans le rapport de match, à un clic.

**Le nombre de spectateurs.** « 15 000 spectateurs » dans la maquette était une
invention : aucune table ne le porte. `PostMatchSequenceStarted.dedicated_fans`
est le facteur de supporters, pas une affluence. **La mention disparaît.**

**Le tier de la dotation de départ**, déjà retiré en phase 1.

## L'aiguillage des onglets

`teams-team-detail.html` n'a qu'une route et trois `<div>` inertes (phase 2). Le
contrôleur suit le patron d'`admin_page.rs` :

```rust
pub async fn team_detail(…) -> Response      // active_tab = "squad"
pub async fn team_treasury_tab(…) -> Response // active_tab = "treasury"

// aiguillage commun
let content = match active_tab {
    "treasury" => render_treasury(…).await,
    _          => render_squad(…),
};
```

Une constante de route de plus :

```
TEAM_TREASURY  "/app/{space_id}/teams/{team_id}/tresorerie"
```

`space_scope` la couvre sans rien ajouter : elle porte `{team_id}`, dont
`teams` déclare déjà le résolveur (`infrastructure/teams/space_ownership.rs`).

**Le contrôle d'accès est celui de la fiche équipe, inchangé.** La trésorerie
n'est pas plus sensible que la valeur d'équipe, déjà affichée dans l'en-tête à
qui voit la page.

## Le service d'assemblage

Les DTOs du port ne touchent ni le contrôleur ni le gabarit (`CLAUDE.md`,
« Domain services pour données inter-BCs »). Un service de la couche
`use_cases/` fait la jonction :

```rust
// teams/use_cases/treasury_statement_service.rs
// arch:no-instrument — service de lecture : assemble une vue, sans intention métier
pub async fn build_statement(
    team_id: &TeamId,
    repo: &dyn ITeamRepository,
    squad: &dyn ISquadPort,
    match_ctx: &dyn IMatchContextPort,
) -> Result<TreasuryStatement, …>
```

Il lit le grand livre une fois, l'effectif une fois, et n'appelle
`find_match_context` que pour les `match_report_id` **distincts** rencontrés —
une saison en compte une poignée, et deux lignes du même match ne doivent pas
produire deux lectures.

Le marqueur `arch:no-instrument` est obligatoire : l'axe 11 de `check-arch`
exige que toute `pub async fn` de `use_cases/` soit instrumentée ou se déclare
sur place, et celle-ci n'est pas un use case.

## Ce que le back ne fait pas

- **Aucune écriture.** La vue est en lecture seule ; aucun use case de commande,
  aucun événement, aucune migration.
- **Aucune pagination** (phase 2).
- **Aucun recalcul** : `balance_after_kpo` est écrit à l'append, il se lit.

## Règles métier

**Aucune à préciser** — confirmé en phase 2, et cette phase n'en fait pas
apparaître. La vue donne à voir des mouvements que d'autres fonctionnalités ont
produits.
