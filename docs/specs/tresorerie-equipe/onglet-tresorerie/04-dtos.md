# Onglet Trésorerie · Phase 4 : contrats de données

**Phase 3** : `03-back.md`

## Entrée : rien à désérialiser

Deux routes, deux GET, aucun corps, aucun paramètre de requête.

```rust
Path((space_id, team_id)): Path<(String, String)>
```

**Aucun DTO d'entrée.** Pas de filtre, pas de tri, pas de curseur — la phase 2 a
écarté la pagination, et rien sur cet écran ne se paramètre. Un `struct Query`
vide serait un contrat qui promet une extension qu'on n'a pas décidée.

*Émis par* le clic sur l'onglet, ou le chargement direct de l'URL · *consommé
par* `team_treasury_tab`.

## Sortie : un fragment, un view model

```rust
#[derive(Template)]
#[template(path = "teams-treasury-tab.html")]
pub struct TreasuryTabTemplate {
    pub app_routes: AppRoutes,
    pub vm: TreasuryVm,
}
```

Le fragment ne porte **ni l'en-tête d'équipe ni les onglets** : ils restent dans
`teams-team-detail.html`, qui n'est pas re-rendu au changement d'onglet
(`hx-target="#team-tab-content"`, phase 2).

Le rendu **complet** de l'URL — quelqu'un qui colle le lien — passe par le même
aiguillage que `admin_page.rs` : la page entière, avec `active_tab = "treasury"`,
qui inclut ce même fragment. **Un seul gabarit pour les deux chemins**, comme
l'administration de compétition le fait déjà.

### Le view model

```rust
pub struct TreasuryVm {
    pub summary: SummaryVm,
    pub groups: Vec<GroupVm>,
    /// Aucun mouvement au-delà de la dotation : le gabarit rend le bloc
    /// « Aucun mouvement pour l'instant » au lieu du tableau.
    pub is_opening_only: bool,
    pub movement_count: u32,
}

pub struct SummaryVm {
    pub opening_kpo: u32,      // la dotation de départ
    pub credited_kpo: u32,     // encaissé, dotation exclue
    pub debited_kpo: u32,      // dépensé
    pub balance_kpo: u32,      // le solde courant
}

/// Un groupe de mouvements : une journée, ou l'ouverture du relevé.
pub struct GroupVm {
    /// `None` pour le groupe d'ouverture — celui de la dotation, qui n'a pas
    /// de journée. Le gabarit n'affiche alors aucun séparateur.
    pub heading: Option<String>,   // « Journée 1 — contre les Trolls du Bief »
    pub rows: Vec<MovementRowVm>,
}

pub struct MovementRowVm {
    pub date_label: String,        // « 12 août »
    pub icon: &'static str,        // l'emoji du motif
    pub label: String,             // « Recrutement »
    pub detail: Option<String>,    // « Gwenn, Passeuse — n° 7 »
    pub amount_label: String,      // « −90 kPo », signe compris
    pub balance_label: String,     // « 380 kPo »
    pub kind: RowKind,
}

/// Ce que la ligne est, pas comment elle s'affiche. Le gabarit en tire ses
/// classes ; c'est lui qui décide de la couleur, pas le view model.
pub enum RowKind {
    Opening,      // la dotation — ouvre le relevé
    Credit,
    Debit,
    Correction,   // MatchIncomeReverted, InducementRefunded
}
```

**`RowKind` et non un `css_class: String`.** Un view model qui porte des noms de
classes CSS fige la présentation dans le code Rust : changer une couleur
demanderait de recompiler. L'énumération dit *ce que la ligne est* ; le gabarit
choisit `tr-row--fix` ou `tr-icon--credit`.

**`amount_label` porte son signe, `balance_label` non.** Le signe appartient au
montant — « −90 kPo » — alors qu'un solde est un état, jamais signé. Les deux
sont des chaînes déjà formatées parce que la mise en forme d'un nombre est une
décision de présentation, prise une seule fois, au même endroit.

**`detail: Option<String>` et non `String`.** Trois motifs n'ont aucun détail
(coups de pouce, remboursement, dotation). Une chaîne vide obligerait le gabarit
à tester `!= ""` — ce qu'Askama fait mal — et laisserait un `<div>` vide qui
prend sa marge.

### Construction

`TreasuryVm` **dépend d'un port** (`IMatchContextPort`) autant que du domaine.
Il ne peut donc pas exposer un `from_domain()` : sa construction vit dans
`builders.rs`, conformément au `CLAUDE.md`.

```rust
// teams/io/web/builders.rs
pub fn build_treasury_vm(statement: &TreasuryStatement) -> TreasuryVm;
```

Le `TreasuryStatement` que rend le service d'assemblage (phase 3) est déjà
purgé des DTOs de port : le builder ne voit ni `MatchContextDto`, ni
`SquadMemberDto`, ni `TreasuryMovementRow`.

## Le contrat du port

```rust
// teams/ports.rs
#[async_trait]
pub trait IMatchContextPort: Send + Sync {
    async fn find_match_context(&self, match_report_id: &str) -> Option<MatchContextDto>;
}

/// DTO de lecture — primitives assumées.
pub struct MatchContextDto {
    pub round_name: String,
    pub home_team_name: String,
    pub away_team_name: String,
    pub home_score: Option<u8>,
    pub away_score: Option<u8>,
}
```

*Implémenté par* `infrastructure/teams/match_context_adapter.rs`, qui lit
`competition_match_display_proj` · *consommé par*
`treasury_statement_service` · *jamais vu* par le contrôleur ni le gabarit.

**`Option` sur la méthode, et `Option` sur les scores** : deux absences
différentes, à ne pas confondre.

- La méthode rend `None` quand le match n'a **aucune ligne d'affichage** —
  typiquement un rapport créé manuellement, tant que la carte 427 n'est pas
  livrée. La ligne de trésorerie s'affiche alors sans journée et sans
  adversaire.
- Les scores sont `None` quand le match **est en cours** : la ligne existe, la
  journée et l'adversaire sont connus, le résultat non.

Sans cette distinction, un match en cours serait traité comme un match inconnu,
et perdrait son en-tête de journée alors qu'on le connaît.

## La sortie du service d'assemblage

```rust
// teams/use_cases/treasury_statement_service.rs
pub struct TreasuryStatement {
    pub opening: Movement,
    pub movements: Vec<Movement>,
    pub balance: Kpo,
}

pub struct Movement {
    pub direction: MovementDirection,   // le type du domaine
    pub reason: MovementReason,         // le type du domaine, pas une String
    pub amount: Kpo,
    pub balance_after: Kpo,
    pub occurred_at: DateTime<Utc>,
    pub detail: Option<String>,
    pub match_context: Option<MatchContext>,
}

pub struct MatchContext {
    pub round_name: String,
    pub opponent_name: String,          // déjà résolu : l'adversaire, pas les deux camps
    pub score: Option<(u8, u8)>,        // (nous, eux)
}
```

**`MovementReason` et non `String`.** Le dépôt rend un `reason: String` — c'est
un DTO de lecture, la primitive y est admise — mais le service le repasse dans
le type du domaine dès qu'il le peut. Un motif inconnu de l'énumération est une
**erreur**, pas une ligne à ignorer : le relevé doit refuser de mentir sur un
solde plutôt que de sauter une ligne.

**`opponent_name` et non les deux noms d'équipes.** Le port rend `home` et
`away` parce qu'il ne sait pas de quelle équipe on parle ; le service, lui, le
sait — il choisit, et le reste de la chaîne n'a plus à comparer des
identifiants. Même chose pour le score, réordonné en (nous, eux).

C'est exactement le rôle que le `CLAUDE.md` donne au domain service :
transformer les DTOs du port en objets du BC consommateur, pour que ni le
contrôleur ni le gabarit ne les voient.

## Le formatage, et où il se décide

| Ce qui est formaté | Où | Pourquoi |
|---|---|---|
| « 12 août » | `builders.rs` | présentation |
| « −90 kPo », « 510 kPo » | `builders.rs` | présentation |
| « Recrutement » (les 8 libellés) | `builders.rs` | présentation |
| « Journée 1 — contre les Trolls du Bief » | `builders.rs` | présentation, à partir de `MatchContext` |
| « Victoire », « Défaite », « Match nul » | `builders.rs` | présentation, à partir du résultat |
| l'emoji du motif | `builders.rs` | présentation |

**Rien n'est formaté ailleurs.** Ni dans le service, qui rend des types, ni dans
le gabarit, qui n'a aucune logique. C'est ce qui rendra la traduction possible
le jour venu (carte 395) : un seul fichier à toucher pour cet écran.

## Règles métier

**Aucune à préciser** — troisième phase consécutive sans. C'est cohérent : la
vue ne décide de rien, elle donne à lire des mouvements déjà écrits.
