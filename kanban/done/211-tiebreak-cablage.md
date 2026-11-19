# Départages — Câblage complet : agrégat, use case, handlers, formulaire

**Priorité : haute**
**Dépend de :** cartes 209 (port catalogue) **et** 210 (VOs + erreurs)
**Contexte :** `src/app/competitions/domain/competition_rules.rs`, `use_cases/save_competition_rules.rs`, `io/web/new_competition.rs`, `templates/new-competition-phase-2.html`, `assets/static/css/pages/new-competition-phase-2.css`
**Spec :** `docs/specs/ranking/tiebreakers/competition-rules-form/{04-dtos,05-use-cases,07-integration}.md`

## Objectif

Basculer la configuration des départages de l'ancien `HashMap<String, u32>` vers
`TiebreakConfig`, et livrer les cases à cocher dans le formulaire. **Carte atomique** :
le changement de forme du champ casse simultanément le domaine, le use case, les
handlers, le template et quatre sites de test — le découper produirait un état
non compilable.

## Conception

### 1. Agrégat (cf. `04-dtos.md`)

Dans `RankingRules` (`competition_rules.rs:52`) :

```rust
- pub additionnal_ranking_points: HashMap<String, u32>,
+ pub tiebreakers: TiebreakConfig,
```

Renommage assumé : l'ancien nom est un contresens (ce ne sont pas des points
additionnels). **Aucune clause de compatibilité** — le projet n'est pas en production,
contrairement au cas `diff_td` de la feature `ranking-bonus-points`.

Forme JSON cible :

```json
"tiebreakers": [
  { "code": "diff_td", "activated": true },
  { "code": "nb_td",   "activated": false }
]
```

L'ordre du tableau **est** la priorité — pas de champ `priority`.

### 2. Use case (cf. `05-use-cases.md`)

`execute` gagne un 3ᵉ paramètre `catalog: &dyn ITiebreakCatalogPort`. La commande est
inchangée.

`execute` fait aujourd'hui **20 lignes pile** (`save_competition_rules.rs:33-52`) :
ajouter la vérification en ligne franchirait la limite du CLAUDE.md. Deux fonctions
nommées sont extraites :

```rust
fn ensure_roster_unicity(rules: &CompetitionRules) -> Result<(), SaveCompetitionRulesError>
fn ensure_known_tiebreak_codes(config: &TiebreakConfig,
                               catalog: &dyn ITiebreakCatalogPort)
    -> Result<(), SaveCompetitionRulesError>
```

`ensure_known_tiebreak_codes` lève `UnknownTiebreakCriterion { code }` au premier code
inconnu. Elle ne vérifie **pas** l'exhaustivité : une configuration qui omet un critère
du catalogue est valide (règle 8).

`SaveCompetitionRulesError` gagne `NoActiveTiebreaker`, `DuplicateTiebreakCode { code }`,
`UnknownTiebreakCriterion { code }`.

### 3. Handlers (cf. `07-integration.md`)

Les deux handlers sont **déjà hors limite** avant nos ajouts — on les découpe puisqu'on
les modifie :

| Handler | Taille actuelle | Extraction |
|---|---|---|
| `get_new_competition_phase_2` (`:48`) | 38 lignes | `load_existing_rules_json`, `load_season_name`, `tiebreak_catalog_json` — le `tokio::join!` reste dans le handler |
| `post_competition_rules` (`:418`) | 45 lignes | `map_save_rules_error(e) -> Response` |

`NewCompetitionPhase2Template` gagne `tiebreak_catalog_json: String`.

Messages 422, dans le style de `RosterInMultipleTiers` :

| Erreur | Message |
|---|---|
| `NoActiveTiebreaker` | « Au moins un critère de départage doit être actif. » |
| `DuplicateTiebreakCode` | « Le critère de départage « … » est présent plusieurs fois. » |
| `UnknownTiebreakCriterion` | « Le critère de départage « … » est inconnu. » |

Le front les affiche déjà tels quels dans `#rules-error-banner` — rien à ajouter.

### 4. Formulaire (`new-competition-phase-2.html`)

| Élément | Changement |
|---|---|
| `TIEBREAK_CRITERIA` (`:164`) | **supprimée** — `JSON.parse` du catalogue injecté |
| `criteriaOrder` (`:174`) | `{ code, label, activated }` au lieu de `{ id, label }` |
| `renderTiebreaks()` (`:177`) | Ligne en `<label>` (libellé cliquable, plus d'avertissement a11y), case à cocher, classe `is-off`, rang « — » si inactif, **numérotation sur les actifs seulement** |
| Drag & drop (`:186-214`) | Principe inchangé — l'ordre porte sur la liste complète, inactifs compris |
| `buildJSON()` (`:373`) | Produit le tableau `tiebreakers` |
| `initFromExistingRules()` (`:446`) | Hydrate ordre **et** activation ; complète depuis le catalogue les critères absents, **actifs** |
| Garde-fou règle 1 | Bouton « Enregistrer & continuer » désactivé + message inline dès zéro critère actif |

Décocher bascule le flag **sur place**, sans déplacer la ligne : c'est ce qui satisfait
la règle 2 par construction.

### 5. CSS (`new-competition-phase-2.css`)

- **ajout** `.tiebreak-check` (calqué sur `.bonus-check`) et `.tiebreak-row.is-off`
- **suppression** `.tiebreak-remove` / `:hover` (lignes 71-72) — code mort, le template
  n'a jamais eu de bouton ✕

Reprendre à l'identique ce qui est validé dans la maquette
`assets/rawpages/html/app-league-rules.html`.

### 6. Sites de test à adapter

| Site | Adaptation |
|---|---|
| tests de `save_competition_rules` | Faux catalogue exposant les 7 codes |
| `base_rules()` (`save_competition_rules.rs:150`) | `additionnal_ranking_points: HashMap::new()` → `tiebreakers` |
| `rules()` (`rules_labels.rs:44`) | Idem |
| `legacy_rules_without_new_fields_deserialize_with_defaults` (`competition_rules.rs:121`) | JSON de fixture : `additionnal_ranking_points` → `tiebreakers` sous la nouvelle forme |

## Checklist

- [ ] `RankingRules.tiebreakers: TiebreakConfig` remplace `additionnal_ranking_points`
- [ ] Use case étendu du port ; `ensure_roster_unicity` et `ensure_known_tiebreak_codes` extraites ; `execute` ≤ 20 lignes
- [ ] 3 nouvelles variantes d'erreur mappées en 422 avec messages français
- [ ] Les deux handlers découpés, chacun ≤ 20 lignes
- [ ] Catalogue injecté dans le template ; `TIEBREAK_CRITERIA` supprimée
- [ ] Case à cocher, état `is-off`, renumérotation des actifs, garde-fou de submit
- [ ] CSS ajouté ; `.tiebreak-remove` supprimé
- [ ] Les 4 sites de test adaptés
- [ ] Aucune migration (colonne `rules` déjà JSONB)
- [ ] `make test` + `make check-arch` passent
