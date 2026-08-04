# Phase 6 — Domaine — page "Mes équipes"

## Récapitulatif exhaustif des règles métier (validé)

1. **Une équipe soumise quitte définitivement la liste `team_creation`** — dès
   `submitted_at` renseigné, elle n'apparaît plus jamais dans "En cours de
   création", quel que soit son statut final côté `teams`.
2. **Groupement actives/archivées** : actives =
   `ParticipationStatus ∈ {PendingEnrollment, Enrolled}` (tout `game_phase`) ;
   archivées = `ParticipationStatus ∈ {Rejected, Dismissed}`.
3. **Libellés/couleurs canoniques** : le badge d'une équipe reprend
   exactement le mapping déjà utilisé sur la page de détail d'équipe
   (`team_detail.rs::status_display`) — pas de nouveau vocabulaire propre à
   cette page.
4. **Pas de filtre, pas de pagination** sur "Mes équipes" — un seul widget,
   tout chargé d'un coup.
5. **Roster non affiché si pas encore choisi** : un brouillon encore au
   stade "ruleset" (roster pas sélectionné) n'affiche pas de tag roster.

## Décision qui clôt la question ouverte depuis la Phase 2

"Archivée" ne devient pas un nouveau concept domaine. `ParticipationStatus`
reste inchangé (`PendingEnrollment` / `Enrolled` / `Dismissed` / `Rejected`)
— "archivée" n'existe qu'au niveau de la présentation, comme un regroupement
calculé par le handler du widget (règle 2 ci-dessus), pas comme une valeur
stockée ou un événement domaine. Aucune méthode domaine, aucun value object,
aucune `DomainError` à ajouter.

## Tests prévus

Pas de test domaine (aucune règle domaine nouvelle). Un test unitaire sur la
fonction pure `status_label_and_class()` (widget, pas domaine) couvrant les
11 lignes du tableau de mapping (phase 4) — c'est la seule logique de
classification à vérifier.
