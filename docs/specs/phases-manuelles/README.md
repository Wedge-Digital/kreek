# Ouvrir une phase manuellement

**Épic :** aucune pour l'instant · **Maquette :**
`assets/rawpages/html/app-team-phase-override.html`

## La fonction

Un commissaire — admin d'espace ou de compétition — ouvre, depuis une équipe
prête à jouer, l'une des trois phases d'après-match : **dépense des SPP**,
**recrutement** ou **renvois**. L'équipe y fait ce qu'elle ferait après un
match ; à la fermeture, elle redevient prête à jouer, **sans enchaîner sur les
autres phases ni sur les erreurs coûteuses**.

C'est un outil de correction : un coach qui a validé ses évolutions sans
dépenser ses SPP, un recrutement oublié, un renvoi décidé hors match.

**Prérequis : carte 570** — un seul service pour savoir qui est admin (livrée).

## Les pages

| Page | Front | Back | DTOs | Use cases | Domaine | Intégration | Cartes |
|---|---|---|---|---|---|---|---|
| `bandeau-fiche-equipe/` | ✅ | ✅ | ✅ | ✅ | | | |

## État du workflow — 2026-10-03

Phase 3 reprise après la livraison de la carte 570 : le droit passe par
`team_admin_guard::require_team_admin` et `is_team_admin`, sans fonction neuve.
Phase 6 en cours.

## Les décisions de conception déjà prises

**L'entrée de phase, à côté de la phase.** L'agrégat garde `game_phase` tel
quel et gagne un champ indépendant :

```rust
#[derive(Default)]
pub enum PhaseEntry {
    #[default]
    PostMatch,   // le cas d'aujourd'hui, sans rien changer
    Override,    // posée par les trois ouvertures manuelles
}
```

Rien de ce qui lit `game_phase` n'a à le connaître — gardes des commandes,
paniers indexés par phase, port de `players`, libellés. Seules les trois
commandes de sortie (`validate_improvement_phase`, `validate_recruitment_phase`,
`validate_dismissals_phase`) et l'éligibilité à la correction d'un rapport le
consultent. Pas de migration : la valeur par défaut couvre toutes les équipes
existantes au rejeu. `PostMatchSequenceStarted` la remet à `PostMatch`, faute
de quoi la phase suivant le prochain vrai match serait prise pour une phase
manuelle.

**Pas de refonte des transitions.** Elles restent faites et vérifiées dans
chaque commande, par `expect_phase`.

**La carte 46 et `GamePhaseOverridden` sont abandonnés** au profit de cette
fonction, plus étroite : trois phases et non toutes, un retour garanti à « prête
à jouer ».

**La projection `team_proj` passe à une liste exhaustive** d'événements, sans
`_ =>` — le piège des cartes 175 et 408, refermé au moment où l'on ajoute quatre
événements.

## Ce que la fonctionnalité suppose déjà acquis

- **La dépense de SPP lit la phase en direct** : `team_roster_adapter` répond
  `in_player_improvement_phase` depuis l'agrégat. Une équipe en
  `PlayerImprovement` peut dépenser, quelle que soit la manière dont elle y est
  entrée.
- **Un match exige `ReadyToPlay`** (`ref_team_data_adapter`) : une équipe en
  phase ne peut pas être sélectionnée.
- **Les trois sorties ramènent déjà à la fiche** (`validate_phase_actions.rs`) :
  rafraîchissement pour les évolutions, redirection depuis leur page pour le
  recrutement et les renvois.
