# Phase 6 — Domaine (`competition-rules-form`)

## Récapitulatif exhaustif des règles métier — validé

**Configuration des départages** (unité `competition-rules-form`)

1. Au moins un critère coché — une configuration entièrement décochée est refusée.
2. L'ordre d'un critère décoché est conservé ; le recocher le remet à sa place.
3. Défaut : les 7 critères actifs, dans l'ordre canonique du catalogue.
4. Activation et ordre sont figés après le démarrage de la compétition.
5. Pas de doublon de code dans une configuration.
6. Configuration non vide.
7. Tout code soumis doit appartenir au catalogue.
8. Une configuration n'a pas besoin d'être exhaustive : un critère omis est valide, il
   sera complété à l'hydratation.

**Catalogue**

9. Le catalogue appartient au BC `ranking` ; `competitions` ne stocke que le choix du
   gestionnaire.
10. 7 critères. `nb_red_cards` est exclu faute de source de données dans
    `MatchActionType`.

**Calcul du départage** (unité `tiebreak-calc`)

11. Un compteur cumulé par critère et par équipe, mis à jour à chaque match publié,
    porté par la ranking line.
12. Les compteurs sont accumulés pour **tous** les critères, indépendamment de
    l'activation ; celle-ci ne joue qu'à l'ordonnancement.
13. `diff_td` est dérivé (`nb_td − nb_td_conceded`), pas stocké.
14. `nb_cas` = actions `Sortie` strictement — même définition que le bonus agressif.
15. `nb_reu` = actions `Passe` uniquement.
16. `nb_fouls` = actions `Agression`.
17. Sens décroissant pour tous les critères, sauf `nb_td_conceded` en croissant.
18. Ordonnancement : `ranking_points` d'abord, puis les critères actifs dans l'ordre de
    priorité jusqu'au premier qui départage.
19. Ex æquo résiduels assumés — aucun départage ultime.

## Portée du domaine pour cette unité

| Règle | Portée |
|---|---|
| 1, 5, 6 | **Domaine `competitions`** — invariants de `TiebreakConfig::try_new()` |
| 2 | Satisfaite par construction (bascule du flag sur place, cf. `02-front.md`) |
| 3 | Amorce front + `TiebreakConfig::all_active(codes)` pour les lecteurs d'une config absente |
| 4 | Déjà vraie de fait — aucune route d'édition des règles après création |
| 7, 8 | Use case, via `ITiebreakCatalogPort` |
| 9, 10 | Domaine `ranking` (catalogue) |
| 11 à 19 | Unité `tiebreak-calc` |

## Correction — pas de `TiebreakConfig::default()`

La phase 5 annonçait un `TiebreakConfig::default()` portant les 7 codes. C'est
**incompatible avec l'option (a)** validée en phase 3 : énumérer les codes dans le
domaine `competitions` lui ferait connaître le catalogue, ce qu'on a explicitement
refusé.

Le constructeur reçoit donc les codes de l'extérieur :

```rust
/// Tous les critères fournis, actifs, dans l'ordre reçu. Les codes viennent du
/// catalogue (ITiebreakCatalogPort) — le domaine ne les connaît pas.
pub fn all_active(codes: Vec<TiebreakCode>) -> Result<Self, DomainError>
```

Appelé par le use case ou par l'ACL `competition_info_adapter`. Le domaine reste
ignorant du catalogue ; la règle 3 reste satisfaite.

## Erreurs domaine — création de `competitions/domain/error.rs`

Le BC `competitions` n'a **pas** de `domain/error.rs`, contrairement à `team_creation`,
`players`, `teams` et `match_report`. Ses VOs ne s'appuient que sur les erreurs
générées par nutype, opaques : un `predicate` violé ne dit pas lequel.

On introduit donc le module manquant, conformément au CLAUDE.md (« `DomainError` : enum
exhaustif avec `thiserror` ») :

```rust
// app/competitions/domain/error.rs
#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum DomainError {
    #[error("La configuration de départage est vide.")]
    EmptyTiebreakConfig,
    #[error("Au moins un critère de départage doit être actif.")]
    NoActiveTiebreaker,
    #[error("Le critère de départage « {code} » est présent plusieurs fois.")]
    DuplicateTiebreakCode { code: String },
}
```

`thiserror` fournit `Display`, requis par le chemin de désérialisation ci-dessous.

## Value objects et méthodes

```rust
#[nutype(
    validate(not_empty),
    derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, Display, AsRef)
)]
pub struct TiebreakCode(String);

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TiebreakSetting {
    pub code:      TiebreakCode,
    pub activated: Activated,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(try_from = "Vec<TiebreakSetting>")]
pub struct TiebreakConfig(Vec<TiebreakSetting>);
```

| Méthode | Rôle |
|---|---|
| `try_new(Vec<TiebreakSetting>) -> Result<Self, DomainError>` | Smart constructor : porte les règles 1, 5, 6 |
| `all_active(Vec<TiebreakCode>) -> Result<Self, DomainError>` | Règle 3, codes fournis par l'appelant |
| `settings(&self) -> &[TiebreakSetting]` | Lecture ordonnée — l'index **est** la priorité |

Pas de méthode de mutation : l'enregistrement des règles remplace la configuration en
bloc, il n'existe pas d'opération « activer un critère » isolée.

### Validation traversée à la désérialisation

Un `#[derive(Deserialize)]` nu sur un newtype **ne passerait pas** par `try_new` — les
invariants seraient contournés par tout payload JSON. D'où :

```rust
impl TryFrom<Vec<TiebreakSetting>> for TiebreakConfig {
    type Error = DomainError;
    fn try_from(v: Vec<TiebreakSetting>) -> Result<Self, Self::Error> { Self::try_new(v) }
}
```

combiné à `#[serde(try_from = "Vec<TiebreakSetting>")]`. C'est ce qui rend vraie
l'affirmation de `04-dtos.md` : une configuration invalide est rejetée en 422 **avant**
d'atteindre le use case, y compris si un client contourne le front.

`Serialize` reste un simple derive : un newtype tuple sérialise comme sa valeur
interne, donc directement en tableau JSON — la forme retenue en phase 4.

## Tests unitaires prévus

| Test | Règle |
|---|---|
| `try_new` refuse une liste vide | 6 |
| `try_new` refuse une configuration dont tous les critères sont décochés | 1 |
| `try_new` refuse un code présent deux fois, et nomme le code fautif | 5 |
| `try_new` accepte une configuration valide et **préserve l'ordre reçu** | 2 |
| `all_active` produit autant de réglages que de codes, tous actifs, dans l'ordre | 3 |
| `all_active` refuse une liste de codes vide | 6 |
| Désérialisation d'un tableau JSON valide → ordre et activation préservés | 2 |
| Désérialisation d'un tableau sans aucun critère actif → **échoue** | 1 |
| Sérialisation → tableau JSON `[{code, activated}]`, ordre préservé | phase 4 |
| Aller-retour sérialisation / désérialisation stable | — |

Le `TiebreakCode` hérite des tests nutype implicites (refus de la chaîne vide) ; un test
explicite est ajouté pour tracer la règle.

## Règles métier — état

Aucune règle nouvelle. La phase corrige la localisation de la règle 3 (pas de
`default()` dans le domaine) et introduit le module d'erreurs domaine manquant.
