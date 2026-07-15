# Team state management — Spec index

Gestion des états de l'agrégat équipe (`teams` BC) en fonction du contexte, sur
la page de détail d'équipe. Bandeau d'état contextuel (maquetté dans
`assets/rawpages/html/app-team-detail.html`, Phase 1 déjà validée) + 3 actions
de transition de phase.

Hors périmètre explicite : recrutement (achat joueurs/staff), gestion des
renvois (choix des joueurs), impression PDF réelle — ces actions ne sont que
des liens/boutons de navigation ou des no-op côté client dans cette feature.

## Pages

| Page | Front | Back | DTOs | Use cases | Domaine | Intégration | Cartes |
|---|---|---|---|---|---|---|---|
| team-detail | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ (5 cartes créées) |

*(Phases 2 à 7 traitées en un seul document `team-detail/02-07-conception.md`,
feature à périmètre restreint sur une page unique sans nouveau widget — voir
précédent `player-match-impact/player-report-events`.)*

## Règles métier transverses (identifiées phases 1-6)

- **Machine à états déjà existante** : `GamePhase` et `ParticipationStatus`
  (`src/app/teams/domain/team.rs`) et toutes leurs gardes de transition sont
  déjà implémentées et testées. Cette feature construit la couche
  use case/route/UI par-dessus, elle ne redéfinit pas le domaine sauf sur le
  point ci-dessous.
- **Aucune transition hors du jeu de transitions documenté n'est possible** —
  chaque méthode domaine vérifie l'état courant avant d'émettre un événement.
  La seule transition libre est administrative (`override_phase()` →
  `GamePhaseOverridden`, déjà implémentée), hors périmètre de cette feature.
- **Modification domaine assumée** : `validate_dismissals_phase()` transitionne
  désormais directement vers `ReadyToPlay` (au lieu de `TemporaryRetirement`)
  — simplification temporaire tant que la carte 39 (retraite temporaire) n'est
  pas développée. À revisiter à ce moment-là.
- **Câblage minimal du déclenchement post-match** : un nouveau listener réagit
  à `MatchReportPublished` pour faire entrer l'équipe en phase
  `PlayerImprovement`, avec des valeurs stub (fans_roll=0, treasury_income=0,
  spp_gains=[]) — le calcul réel (revenus, jet de fans, SPP) reste hors
  périmètre, à la charge des cartes 35/145/154.
- **Phases non couvertes par le bandeau** (`TemporaryRetirement`,
  `OffSeason`, statuts `Dismissed`/`Rejected`) : aucun bandeau affiché, seul le
  badge d'en-tête existant reste visible.
- **Agrégat `Team` event-sourcé** : confirmé — source de vérité =
  `team_event_store` rejoué via `Team::hydrate()`, `team_proj` n'est qu'une
  projection de lecture dérivée.
