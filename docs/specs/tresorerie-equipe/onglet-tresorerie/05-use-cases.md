# Onglet Trésorerie · Phase 5 : couche applicative

**Phase 4** : `04-dtos.md`

## Aucun use case de commande, et cette phase est courte pour cette raison

La phase 5 du workflow traite **les mutations**. Cet écran n'en a aucune : pas
de POST, pas de PUT, pas de DELETE, aucun agrégat chargé pour être modifié,
aucun événement émis.

Écrire un `_use_case.rs` pour une lecture serait une erreur de nommage qui
coûterait plus tard : le prochain qui ouvre `use_cases/` doit pouvoir supposer
qu'un fichier en `_use_case.rs` mute quelque chose.

Le travail applicatif de cet écran tient donc en **un service de lecture**, déjà
esquissé en phase 3 et spécifié ici.

## `treasury_statement_service`

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

Le marqueur `arch:no-instrument` est **obligatoire** : l'axe 11 de `check-arch`
exige que toute `pub async fn` de `use_cases/` soit instrumentée ou se déclare
sur place. Le motif qui suit le marqueur l'est aussi — une liste d'exceptions
tenue ailleurs aurait dérivé.

### Orchestration

```
1. repo.list_treasury_movements(team_id)        une requête, jointure comprise
2. squad.find_squad(team_id)                    une requête, indexée par player_id
3. pour chaque match_report_id DISTINCT :
       match_ctx.find_match_context(id)
4. convertir chaque ligne en Movement, résoudre son détail
5. séparer la dotation du reste, calculer les totaux
```

**Trois familles de requêtes, pas une par ligne.** Le point 3 est le seul qui
pourrait déraper : deux lignes du même match — l'achat de coups de pouce et son
remboursement — ne doivent pas produire deux lectures. Les identifiants sont
collectés, dédupliqués, puis résolus.

C'est le genre de N+1 qui ne se voit pas en développement, où un relevé fait six
lignes, et qui se paie sur une saison entière.

### La dotation n'est pas un mouvement comme les autres

Elle ouvre le relevé et alimente `SummaryVm.opening_kpo` ; elle est **exclue**
du total encaissé, qui ne compte que ce que l'équipe a gagné.

Le service la sépare donc explicitement plutôt que de laisser le builder la
reconnaître à son motif :

```rust
pub struct TreasuryStatement {
    pub opening: Movement,          // InitialEndowment
    pub movements: Vec<Movement>,   // tout le reste, dans l'ordre
    pub balance: Kpo,
}
```

**`opening` n'est pas un `Option`, et c'est un choix.** Mesuré le 2026-08-26 :
les **3 258 équipes** de la base ont un grand livre, et **toutes** portent une
ligne `InitialEndowment`. Aucune équipe héritée n'échappe à la règle — le grand
livre a toujours été alimenté depuis la création de l'équipe.

Une dotation absente est donc une **incohérence**, pas un cas nominal, et le
service rend `MissingOpeningEntry` plutôt qu'un relevé qui commencerait dans le
vide. Le solde afficherait sinon un enchaînement qui ne tombe pas juste, sans
que rien ne le signale.

### Le solde vient de la dernière ligne, jamais d'une somme

```rust
balance = movements.last().unwrap_or(&opening).balance_after
```

`balance_after_kpo` est écrit à l'append, dans la transaction de l'événement.
Le **recalculer** par addition introduirait une seconde vérité qui pourrait
diverger de la première — et c'est précisément la divergence qu'un relevé est
censé rendre visible, pas produire.

Les deux totaux de la synthèse, eux, **sont** des sommes — encaissé et dépensé
n'existent nulle part ailleurs. Ils se déduisent de la direction de chaque ligne.

### Les erreurs

```rust
pub enum TreasuryStatementError {
    /// Le grand livre est vide, ou ne commence pas par une dotation.
    MissingOpeningEntry,
    /// Un motif que `MovementReason` ne connaît pas.
    UnknownReason(String),
    Repository(String),
}
```

**`UnknownReason` est un refus, pas un saut de ligne.** Si un neuvième motif
était écrit en base sans être ajouté à l'énumération, sauter la ligne
produirait un relevé dont les soldes ne s'enchaînent plus — un défaut qui se lit
comme une erreur de calcul et se cherche des heures du mauvais côté. Le relevé
doit refuser de s'afficher.

C'est le seul endroit de cet écran où l'on préfère une page en erreur à une page
approximative, et c'est délibéré : un relevé de compte faux est pire qu'un
relevé absent.

### Les ports que le service ne prend pas

`IRosterCatalogPort` **n'est pas dans la signature.** Le poste d'un joueur
arrive déjà résolu par `ISquadPort`, dont le DTO porte `position_name` — le
consulter une seconde fois serait une lecture pour rien.

`IMatchContextPort` **est le seul port nouveau** de toute la fonctionnalité.

## Ce que le service ne fait pas

- **Aucune écriture**, aucun événement, aucune transaction.
- **Aucun formatage** : il rend des types — `Kpo`, `MovementReason`,
  `DateTime<Utc>` — et `builders.rs` les met en forme (phase 4).
- **Aucun filtrage** : le relevé est rendu entier.

## Règles métier

**Aucune à préciser** — quatrième phase consécutive. Les deux seules règles que
cette phase pose sont des règles de **cohérence de lecture**, pas de métier :
une dotation manquante et un motif inconnu arrêtent le rendu au lieu de le
dégrader.
