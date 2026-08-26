# Onglet Trésorerie · Phase 6 : domaine

**Phase 5** : `05-use-cases.md`

## Les règles, récapitulées — et ce qu'elles sont vraiment

Quatre phases de suite ont répondu « aucune règle métier à préciser ». En les
relisant d'un bloc, voilà ce qui a été posé, classé par nature.

### Règles de lecture — comment on restitue

| | Règle | Posée en |
|---|---|---|
| L1 | La dotation ouvre le relevé et **n'entre pas** dans l'encaissé | phase 4 |
| L2 | L'ordre est celui du cumul : `event_version`, jamais `occurred_at` | phase 3 |
| L3 | Le solde est **lu** (`balance_after_kpo`), jamais recalculé | phase 5 |
| L4 | L'encaissé et le dépensé sont **sommés** — ils n'existent nulle part ailleurs | phase 5 |

L2 mérite son motif : deux mouvements d'un même traitement — l'achat de coups de
pouce et la recette du même match — partagent l'horodatage à la milliseconde.
Seule la version de l'agrégat porte l'ordre dans lequel les soldes s'enchaînent.

### Règles de cohérence — quand refuser d'afficher

| | Règle | Posée en |
|---|---|---|
| C1 | Une dotation absente arrête le relevé (`MissingOpeningEntry`) | phase 5 |
| C2 | Un motif inconnu de l'énumération arrête le relevé (`UnknownReason`) | phase 5 |

Les deux choisissent l'échec plutôt que l'approximation. Sauter une ligne
produirait des soldes qui ne s'enchaînent plus — un défaut qui se lit comme une
erreur de calcul et se cherche du mauvais côté. **Un relevé de compte faux est
pire qu'un relevé absent.**

### Règles de repli — quand afficher moins

| | Règle | Posée en |
|---|---|---|
| R1 | Un joueur renvoyé perd son nom : repli sur le poste, qui vient de l'événement | phase 1 |
| R2 | Un match sans ligne d'affichage perd sa journée et son adversaire | phase 3 |

R2 est la dépendance à la **carte 427** : un rapport créé manuellement n'a pas
de ligne dans `competition_match_display_proj` tant qu'il n'est pas publié. La
427 livrée, ces lignes se complètent sans qu'on y revienne.

### Ce que ce classement montre

**Aucune de ces huit règles n'est un invariant de domaine.** Aucune ne répond à
« est-ce autorisé ? » ni à « que se passe-t-il quand ? ». Elles disent comment
restituer, quand refuser de restituer, et comment restituer moins.

C'est cohérent avec ce qu'est cet écran : il donne à lire des mouvements que
d'autres fonctionnalités ont écrits. **Le domaine de `teams` n'a pas à changer
pour ça** — il lui manque seulement de quoi se relire.

## L'addition : l'inverse d'`as_str`

`teams/domain/treasury.rs` porte déjà tout ce qu'il faut :
`MovementDirection`, `MovementReason` et ses huit variantes, `TreasuryMovement`.

Il lui manque une seule chose, et son absence raconte l'histoire du grand livre :

```rust
impl MovementReason {
    pub fn as_str(&self) -> &'static str { … }   // existe depuis l'origine
    // pas d'inverse
}
```

**`as_str` existe parce qu'on écrit. L'inverse n'existe pas parce que personne
n'a jamais lu** (phase 3 : les seules lectures du grand livre sont dans les
tests du dépôt).

### La forme retenue

```rust
impl MovementReason {
    /// La table qui fait foi pour la lecture. Un motif absent d'ici est un
    /// motif que le relevé refusera — voir C2.
    const ALL: [(MovementReason, &'static str); 8] = [
        (Self::InitialEndowment,    "InitialEndowment"),
        (Self::MatchIncome,         "MatchIncome"),
        (Self::MatchIncomeReverted, "MatchIncomeReverted"),
        (Self::CostlyMistake,       "CostlyMistake"),
        (Self::InducementPurchase,  "InducementPurchase"),
        (Self::InducementRefunded,  "InducementRefunded"),
        (Self::PlayerRecruitment,   "PlayerRecruitment"),
        (Self::StaffPurchase,       "StaffPurchase"),
    ];

    /// Inchangé : un `match` exhaustif, donc vérifié par le compilateur.
    pub fn as_str(&self) -> &'static str { … }

    /// Nouveau, dérivé de `ALL`.
    pub fn parse(raw: &str) -> Option<Self>;
}
```

Même chose pour `MovementDirection`, dont les deux variantes ne justifient pas
de table : un `match` sur `"Credit"` et `"Debit"` suffit.

### Pourquoi `as_str` ne dérive pas de `ALL`

Ce serait le troisième endroit en moins, et c'est tentant. On ne le fait pas
pour deux raisons :

- **Le `match` exhaustif est vérifié par le compilateur.** Ajouter une variante
  casse la compilation d'`as_str`, ce qui force à y penser. Une recherche dans
  `ALL` compilerait sans rien dire.
- Elle imposerait un `unwrap()` sur une recherche qui ne peut pas échouer —
  c'est-à-dire une panique en production pour un cas que le type interdit.

### Le trou que ça laisse, dit franchement

**Ajouter une variante sans l'ajouter à `ALL` n'est pas attrapé par le
compilateur.** Le compilateur force à toucher `as_str` ; il ne dit rien de
`ALL`.

Le filet est un test d'aller-retour. Il n'est pas décoratif : c'est **le seul**
mécanisme qui relie l'énumération à sa table.

Trois endroits à toucher pour un nouveau motif — l'énumération, `as_str`,
`ALL` — et un test qui le rappelle quand on en oublie un.

## Tests

| Test | Règle |
|---|---|
| `tous_les_motifs_font_l_aller_retour` | `parse(as_str(v)) == Some(v)` pour chaque entrée de `ALL` — **le test qui garde la table** |
| `parse_refuse_un_motif_inconnu` | C2 : `parse("Pillage") == None` |
| `parse_est_sensible_a_la_casse` | `parse("matchincome") == None` — les motifs sont écrits par `as_str`, jamais saisis |
| `les_deux_directions_font_l_aller_retour` | même garantie pour `MovementDirection` |

`tous_les_motifs_font_l_aller_retour` est celui qui compte. Sans lui, une
neuvième variante ajoutée à l'énumération et à `as_str` mais oubliée dans `ALL`
produirait un `UnknownReason` en production — sur une ligne que le code venait
tout juste d'apprendre à écrire.

## Ce que la phase n'ajoute pas

- **Aucun agrégat, aucune méthode d'agrégat.** `Team` n'est pas touché.
- **Aucun value object.** `Kpo` existe, `MovementReason` existe.
- **Aucune variante de `DomainError`.** Les deux refus de cohérence — dotation
  absente, motif inconnu — sont des erreurs **applicatives**
  (`TreasuryStatementError`, phase 5), pas des violations d'invariant : elles
  décrivent une base incohérente, pas une commande interdite.
