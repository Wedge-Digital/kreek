# Bandeau de la fiche équipe · Phase 4 : contrats de données

## Entrée — le formulaire

Format HTTP : les primitives sont assumées, le contrôleur les traduit.

```rust
#[derive(Debug, Deserialize)]
pub struct PhaseOverrideForm {
    pub phase: String,          // "player_improvement" | "recruitment" | "dismissals"
    #[serde(default)]
    pub reason: String,         // vide = pas de motif
}
```

Les valeurs envoyées sont **en anglais** (règle 18) : ce sont des identifiants
lus par le navigateur, pas des libellés.

## La commande

Aucune primitive nue.

```rust
pub struct OpenPhaseOverrideCommand {
    pub team_id: TeamId,
    pub phase: OverridablePhase,
    pub reason: Option<OverrideReason>,
    pub admin_id: CoachId,
    pub admin_name: CoachName,
}
```

- **`OverridablePhase`** — énumération du domaine : `PlayerImprovement`,
  `Recruitment`, `Dismissals`, et rien d'autre. Une phase qu'on ne peut pas
  ouvrir manuellement n'est pas exprimable. Le contrôleur la construit par
  `OverridablePhase::parse(&str) -> Option<Self>` ; une valeur inconnue rend
  **400** — elle ne vient pas de l'écran.
- **`OverrideReason`** — value object sur le modèle d'`AdjustmentNote` : `trim`,
  **200 caractères au plus**, charset `TEXTE_SAISI`. Un champ vide donne `None` :
  le motif est facultatif. Un motif refusé affiche un message au pied du
  panneau.
- **`admin_name`** est conservé à côté d'`admin_id`, comme pour
  `TreasuryAdjusted` : la maquette annonce que l'ouverture « est enregistrée
  avec votre nom et le motif ».

## Sorties — les view models

```rust
/// Le bouton du bandeau — présent seulement si l'équipe est prête à jouer et
/// que le visiteur est admin (`is_team_admin`).
BannerCtaVm::OpenPhaseOverride { post_url: String, choices: Vec<PhaseChoiceVm> }

/// Une carte du panneau, construite depuis `OverridablePhase::ALL`.
pub struct PhaseChoiceVm {
    pub value: &'static str,       // "recruitment"
    pub label: &'static str,       // "Recrutement"
    pub description: &'static str,
    pub icon: &'static str,        // "🛒"
    pub open_label: &'static str,  // "Ouvrir le recrutement"
}

/// Le pied du panneau en cas de refus.
pub struct PhaseOverrideErrorVm { pub message: String }
```

**Les cartes viennent du view model, pas du gabarit** : la valeur envoyée et la
phase que le serveur sait ouvrir sortent de la même liste. En dur dans le
gabarit, une faute de frappe ne se verrait qu'en 400, sans explication à
l'écran.

## Le résultat du use case

```rust
pub enum OpenPhaseOverrideError {
    TeamNotFound,                 // → 404
    Domain(DomainError),          // l'équipe n'est plus prête à jouer → message au pied
    Repository(RepositoryError),  // → 500
}
```

## Les événements domaine

Détaillés en phase 6.

| Événement | Charge |
|---|---|
| `ManualImprovementPhaseOpened` | `admin_id: CoachId`, `admin_name: CoachName`, `reason: Option<OverrideReason>` |
| `ManualRecruitmentPhaseOpened` | idem |
| `ManualDismissalsPhaseOpened` | idem |
| `ManualPhaseClosed` | `phase: GamePhase` — celle qu'on quitte |

## Qui émet, qui consomme

| DTO | Émis par | Consommé par |
|---|---|---|
| `PhaseOverrideForm` | le panneau (formulaire HTMX) | `post_phase_override` |
| `OpenPhaseOverrideCommand` | `post_phase_override` | `open_phase_override_use_case` |
| `OverridablePhase` | `OverridablePhase::parse` (contrôleur) | la commande, puis `Team::open_phase_override` |
| `OverrideReason` | le contrôleur (smart constructor) | la commande, puis l'événement |
| `OpenPhaseOverrideError` | le use case | le contrôleur, qui choisit la réponse HTTP |
| `BannerCtaVm::OpenPhaseOverride`, `PhaseChoiceVm` | `BannerVm::from_domain` | `teams-team-detail.html` |
| `PhaseOverrideErrorVm` | le contrôleur, sur un refus | `phase-override-foot.html` |
| les quatre événements | l'agrégat `Team` | `apply()`, la projection `team_proj`, `returns_to_ready_to_play()`, le publisher (groupe muet) |

## Règles métier

Question posée le 2026-10-03 — une règle fixée à cette étape :

9. **Le motif fait au plus 200 caractères**, comme celui d'un ajustement de
   trésorerie.
