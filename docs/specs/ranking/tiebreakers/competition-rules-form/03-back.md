# Phase 3 — Architecture back (`competition-rules-form`)

## Mapping BC

Pas de widget (cf. `02-front.md`, D1). Les points d'ancrage sont les deux handlers
existants de la page phase 2 :

| Handler | Fichier | Rôle |
|---|---|---|
| `get_new_competition_phase_2` | `competitions/io/web/new_competition.rs:48` | Sert la page ; doit désormais injecter le catalogue |
| `post_competition_rules` | `competitions/io/web/new_competition.rs:418` | Enregistre les règles ; reçoit la configuration étendue |

`competitions` fournit la page. `ranking` fournit le catalogue, consulté via port.

## Plan de fichiers

| Fichier | Nature | Rôle |
|---|---|---|
| `app/ranking/domain/tiebreak.rs` | **nouveau** | `TiebreakCriterion` (7 variantes), `all()` dans l'ordre canonique, `code()` — identifiant stable en JSON et en persistance |
| `app/ranking/io/web/tiebreak_labels.rs` | **nouveau** | Libellés français, sur le modèle de `competitions/io/web/rules_labels.rs` |
| `app/competitions/ports.rs` | modifié | `TiebreakCriterionDto { code, label }` + `trait ITiebreakCatalogPort` |
| `infrastructure/competitions/tiebreak_catalog_adapter.rs` | **nouveau** | Implémente le port en lisant `ranking` — seul fichier à importer le BC source |
| `app/competitions/context.rs` | modifié | Champ `tiebreak_catalog_port: Arc<dyn ITiebreakCatalogPort>` + paramètre de `new()` |
| `main.rs` | modifié | Instancie l'adapter, l'injecte dans `CompetitionsContext::new` |
| `app/competitions/domain/competition_rules.rs` | modifié | Le `HashMap<String, u32>` stringly-typé cède la place à un VO « ordre + activation » |
| `app/competitions/use_cases/save_competition_rules.rs` | modifié | Vérifie l'appartenance des codes au catalogue via le port |
| `app/competitions/io/web/new_competition.rs` | modifié | Injection du catalogue JSON dans le template |
| `templates/new-competition-phase-2.html` | modifié | `TIEBREAK_CRITERIA` (ligne 164) supprimée ; case à cocher ; ligne en `<label>` |

Le port est **synchrone** et l'adapter **sans état** — pas de repository à injecter,
contrairement à `SkillCatalogAdapter` (`infrastructure/players/skill_catalog_adapter.rs`,
pris comme modèle). Le port n'existe que pour respecter l'interdiction faite à
`competitions` d'importer `ranking` directement.

## Décision — validation de l'appartenance au catalogue (option **a**)

Le domaine `competitions` **ne connaît pas** le catalogue : un domaine n'appelle pas de
port. La répartition est donc :

| Vérification | Couche | Justification |
|---|---|---|
| Le code appartient-il au catalogue ? | **Use case** `save_competition_rules` | Consultation d'une donnée externe via port — responsabilité explicite du use case |
| Au moins un critère actif (règle 1) | **Domaine** `competitions` | Invariant de l'agrégat `RankingRules` |
| Pas de doublon de code | **Domaine** `competitions` | Idem |
| Liste non vide | **Domaine** `competitions` | Idem |

L'alternative écartée était de dupliquer l'énumération des critères dans le domaine
`competitions` : validation complète à la compilation, mais deux sources de vérité pour
un catalogue qui appartient à `ranking`.

## Contrat HTTP — constat à traiter en phase 4

`SaveRulesPayload` (`new_competition.rs:410`) désérialise **directement** l'agrégat
domaine depuis le JSON (`#[serde(flatten)] rules: CompetitionRules`). Le format de fil
et le format persisté (JSONB) sont donc le même type : changer la forme du champ des
départages change les deux d'un coup. La phase 4 arbitre les deux simultanément.

## Ports

```rust
// app/competitions/ports.rs
pub struct TiebreakCriterionDto {
    pub code:  String,
    pub label: String,
}

pub trait ITiebreakCatalogPort: Send + Sync {
    /// Catalogue complet, dans l'ordre canonique.
    fn all(&self) -> Vec<TiebreakCriterionDto>;
}
```

Émis par l'adapter, consommé par le handler GET (injection template) et par le use case
(validation d'appartenance). Jamais exposé au template : le handler le projette en VM /
JSON — cf. phase 4.

## Domain services

Aucun. La transformation DTO → objet domaine est ici triviale (un code, un flag, un
rang) et vit dans le smart constructor du VO ; pas de résolution ni de mapping complexe
justifiant un service dédié.

## Tests prévus

- **Domaine** : refus si tous les critères sont décochés (règle 1), refus si doublon de
  code, ordre préservé au décochage (règle 2), défaut à 7 actifs (règle 3).
- **Use case** : refus si un code inconnu du catalogue est soumis.
- **Adapter** : le catalogue expose 7 entrées, codes stables, ordre canonique respecté.
- **E2E** : spécifié en phase 7.

## Règles métier — état

Aucune règle nouvelle à cette étape pour l'unité 1. Deux décisions prises en phase 3
concernent le catalogue et l'unité `tiebreak-calc`, consignées dans `../README.md` :
retrait de `nb_red_cards` (aucune source de données) et accumulation systématique des
compteurs.
