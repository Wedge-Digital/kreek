# Lire le grand livre d'une équipe

**Épic :** E06 — La fiche d'équipe complétée · **Ordre :** 1 · **Dépend de :** rien
**Conception :** `docs/specs/tresorerie-equipe/onglet-tresorerie/` (`03-back.md`,
`05-use-cases.md`, `06-domaine.md`)

## Objectif

Rendre le grand livre lisible : un relevé correct, prouvé par ses tests, **sans
une ligne de HTML**. C'est là qu'est le risque de la fonctionnalité.

## Le constat

`teams__treasury_ledger` est alimenté depuis l'origine, dans la transaction de
l'append (`team_repository.rs:310`). **Aucun code de production ne le lit** —
les seules lectures sont dans les tests du dépôt.

Son absence de lecteur se voit dans le domaine : `MovementReason::as_str` existe
parce qu'on écrit, **son inverse n'existe pas parce que personne n'a jamais lu**.

## Conception

### 1. Le domaine — l'inverse d'`as_str`

```rust
// teams/domain/treasury.rs
impl MovementReason {
    /// La table qui fait foi pour la lecture.
    const ALL: [(MovementReason, &'static str); 8] = [ … les huit motifs … ];

    pub fn as_str(&self) -> &'static str { … }   // inchangé : match exhaustif
    pub fn parse(raw: &str) -> Option<Self>;     // dérivé de ALL
}
```

**`as_str` reste un `match`** : le compilateur force alors à traiter toute
nouvelle variante, ce qu'une recherche dans `ALL` ne ferait pas, et cela évite
un `unwrap()` sur une recherche qui ne peut pas échouer.

**Le trou, dit franchement** : ajouter une variante sans l'ajouter à `ALL` n'est
pas attrapé par le compilateur. Trois endroits à toucher pour un nouveau motif,
et le test d'aller-retour est **le seul** mécanisme qui les relie.

`MovementDirection` reçoit le même `parse`, sans table — deux variantes.

### 2. Le dépôt — une méthode, une requête

```rust
async fn list_treasury_movements(&self, team_id: &TeamId)
    -> Result<Vec<TreasuryMovementRow>, RepositoryError>;
```

```sql
-- repositories/sql/teams/list_treasury_movements.sql
SELECT l.event_version, l.direction, l.amount_kpo, l.reason,
       l.balance_after_kpo, l.occurred_at, e.payload
FROM   teams__treasury_ledger l
LEFT   JOIN team_event_store e
       ON e.team_id = l.team_id AND e.version = l.event_version
WHERE  l.team_id = $1
ORDER  BY l.event_version
```

**`event_version` est la version de l'agrégat** : la jointure rend l'événement
exact qui a produit le mouvement, donc son détail, sans changer le schéma.

**`LEFT JOIN` et non `JOIN`** : une ligne dont l'événement manquerait doit
s'afficher sans détail plutôt que disparaître — un relevé à trou est pire qu'un
relevé sommaire.

**`ORDER BY event_version` et non `occurred_at`** : deux mouvements d'un même
traitement partagent l'horodatage à la milliseconde ; seule la version porte
l'ordre dans lequel les soldes s'enchaînent.

L'index existe déjà — la contrainte d'unicité `(team_id, event_version)` qui
sert le `ON CONFLICT` de l'écriture couvre exactement ce `WHERE` et cet `ORDER
BY`. **À confirmer d'un `EXPLAIN`**, pas à supposer.

### 3. Le port vers `competitions`

```rust
// teams/ports.rs
#[async_trait]
pub trait IMatchContextPort: Send + Sync {
    async fn find_match_context(&self, match_report_id: &str) -> Option<MatchContextDto>;
}

pub struct MatchContextDto {
    pub round_name: String,
    pub home_team_name: String,
    pub away_team_name: String,
    pub home_score: Option<u8>,
    pub away_score: Option<u8>,
}
```

L'adapter lit `competition_match_display_proj`, où le nom de journée, les deux
équipes et le score vivent **sur une seule ligne**, indexée par
`match_report_id`. Il vit dans `src/infrastructure/teams/match_context_adapter.rs`,
aux côtés des cinq qui y sont déjà, et **il est le seul à importer
`competitions`**.

**Deux `Option` qui ne disent pas la même absence** :
la méthode rend `None` quand le match n'a **aucune ligne d'affichage** — un
rapport créé manuellement, tant que la carte 427 n'est pas livrée ; les scores
rendent `None` quand le match **est en cours**. Confondre les deux ferait perdre
son en-tête de journée à un match qu'on connaît.

### 4. Le service d'assemblage

```rust
// teams/use_cases/treasury_statement_service.rs
// arch:no-instrument — service de lecture : assemble une vue, sans intention métier
pub async fn build_statement(
    team_id: &TeamId,
    repo: &dyn ITeamRepository,
    squad: &dyn ISquadPort,
    match_ctx: &dyn IMatchContextPort,
) -> Result<TreasuryStatement, TreasuryStatementError>
```

Le marqueur est **obligatoire** : l'axe 11 de `check-arch` exige que toute
`pub async fn` de `use_cases/` soit instrumentée ou se déclare sur place.

**Trois familles de requêtes, jamais une par ligne.** Les `match_report_id` sont
collectés, **dédupliqués**, puis résolus : deux lignes du même match — l'achat
de coups de pouce et son remboursement — ne doivent pas produire deux lectures.
C'est un N+1 invisible sur un relevé de six lignes, et coûteux sur une saison.

**Le solde vient de la dernière ligne, jamais d'une somme.** `balance_after_kpo`
est écrit dans la transaction de l'événement ; le recalculer créerait une
seconde vérité qui pourrait diverger de celle que le relevé est censé rendre
visible. L'encaissé et le dépensé, eux, **sont** des sommes — ils n'existent
nulle part ailleurs.

**Le service résout l'adversaire et réordonne le score en (nous, eux)** : le
port ne sait pas de quelle équipe on parle, le service le sait, et le reste de
la chaîne n'a plus à comparer des identifiants.

```rust
pub enum TreasuryStatementError {
    MissingOpeningEntry,
    UnknownReason(String),
    Repository(String),
}
```

**Les deux refus arrêtent le relevé.** Sauter une ligne produirait des soldes
qui ne s'enchaînent plus — un défaut qui se lit comme une erreur de calcul et se
cherche du mauvais côté. Un relevé de compte faux est pire qu'un relevé absent.

`opening` n'est **pas** un `Option` : mesuré le 2026-08-26, les 3 258 équipes de
la base ont un grand livre et **toutes** portent une ligne `InitialEndowment`.

## Le détail, source par source

| Motif | Détail | Source |
|---|---|---|
| `InitialEndowment` | « Création de l'équipe » | libellé fixe |
| `MatchIncome` | « Victoire » / « Défaite » / « Match nul » | `PostMatchSequenceStarted.result` |
| `MatchIncomeReverted` | « Rapport de match corrigé » | libellé fixe |
| `PlayerRecruitment` | « Gwenn, Passeuse — n° 7 » | `ISquadPort` (déjà utilisé) |
| `StaffPurchase` | « Apothicaire × 1 » | `StaffBought.staff_type`, `quantity` |
| `InducementPurchase` | — | abandonné, cf. phase 3 |
| `InducementRefunded` | « Rendus avec l'annulation du rapport » | libellé fixe |
| `CostlyMistake` | l'incident et le jet | `CostlyMistakesApplied` (épic E13) |

**`IRosterCatalogPort` n'est pas dans la signature** : le poste arrive déjà
résolu par `ISquadPort`, dont le DTO porte `position_name`.

**Un joueur renvoyé perd son nom** — `ISquadPort` rend l'effectif courant, pas
l'historique. Repli sur le poste, qui vient de l'événement.

## Tests

| Test | Règle |
|---|---|
| `tous_les_motifs_font_l_aller_retour` | **le test qui garde la table** `ALL` |
| `parse_refuse_un_motif_inconnu` | `parse("Pillage") == None` |
| `parse_est_sensible_a_la_casse` | les motifs sont écrits par `as_str`, jamais saisis |
| `les_deux_directions_font_l_aller_retour` | idem pour `MovementDirection` |
| `list_treasury_movements_rend_l_evenement_joint` | intégration, vraie base |
| `une_ligne_sans_evenement_reste_dans_le_releve` | le `LEFT JOIN` |
| `le_solde_est_celui_de_la_derniere_ligne` | jamais une somme |
| `un_motif_inconnu_arrete_le_releve` | `UnknownReason` |
| `une_dotation_absente_arrete_le_releve` | `MissingOpeningEntry` |
| `deux_lignes_du_meme_match_ne_font_qu_une_lecture` | le N+1 |
| `un_joueur_renvoye_se_replie_sur_son_poste` | le repli |

## Checklist

- [ ] `ALL`, `parse` sur les deux énumérations, et les quatre tests d'aller-retour
- [ ] `list_treasury_movements` + son `.sql` + `EXPLAIN` vérifié
- [ ] `IMatchContextPort`, l'adapter, l'injection dans `main.rs`
- [ ] `treasury_statement_service` et ses sept tests
- [ ] `make lint && make test && make check-arch`
