# BC `teams` — Customisation admin (override d'état)

**Priorité : basse**
**Dépend de :** `28-teams-aggregate.md`
**Contexte :** `teams` — action admin

## Objectif

Permettre à un admin de forcer manuellement la phase de jeu d'une équipe — en avant ou en arrière dans la séquence post-match. Outil de correction pour les situations où une phase a été mal validée, ou pour débloquer une équipe. Future extension : modification directe des joueurs via widget BC `players`.

---

## Ce qui est défini

- **Réservé aux admins** exclusivement — vérification du rôle dans le handler
- **Toutes les transitions sont permises**, y compris revenir en arrière dans la séquence
- La modification manuelle d'état produit un événement dédié dans l'event store (traçabilité)
- Future extension : la fenêtre de customisation accueillera aussi la modification des joueurs (widget BC `players`)

---

## Événements domaine produits

```rust
TeamDomainEvent::GamePhaseOverridden {
    admin_id:   String,
    from_phase: Option<GamePhase>,
    to_phase:   GamePhase,
    reason:     Option<String>,  // commentaire admin optionnel
}
```

L'événement est distinct d'une transition normale — il est toujours auditable séparément dans l'event store.

## Commande

```rust
pub struct OverrideGamePhaseCommand {
    pub team_id:   TeamId,
    pub admin_id:  UserId,
    pub to_phase:  GamePhase,
    pub reason:    Option<String>,
}
```

## Route

```
GET  /app/{space_id}/teams/{team_id}/customise       → panneau de customisation (fragment)
POST /app/{space_id}/teams/{team_id}/override-phase
```

## UI

Le panneau de customisation affiche :
- La phase actuelle de l'équipe
- Un sélecteur de phase cible (toutes les phases disponibles)
- Un champ texte optionnel "Raison" pour documenter la correction
- Bouton "Appliquer"

Future : slots pour les widgets BC `players` (modification directe des joueurs)

---

## Ce qui reste à définir

- Quelles phases sont accessibles depuis le sélecteur ? Toutes les valeurs de `GamePhase`, ou uniquement les phases de jeu (pas `PendingEnrollment`, `Enrolled`, `Dismissed`) ?
- L'override est-il possible uniquement si l'équipe est `Enrolled`, ou aussi dans d'autres statuts de participation ?
- Faut-il une confirmation explicite (modale "Êtes-vous sûr ?") avant d'appliquer ?
- L'historique des overrides est-il visible dans l'UI (journal admin) ?

---

## Checklist (à compléter après raffinage)

- [ ] `TeamDomainEvent::GamePhaseOverridden` + `Team::apply()` mis à jour
- [ ] `OverrideGamePhaseCommand` + use case (vérif rôle admin)
- [ ] `Team::override_phase()` — pas de garde sur la phase courante, toutes les transitions permises
- [ ] Routes GET (panneau) + POST (appliquer)
- [ ] Fragment UI : phase courante + sélecteur + champ raison
- [ ] Vérification rôle admin dans le handler

---

## Note — 2026-07-29 (carte 251)

La TV est désormais recalculée à chaque entrée en `ReadyToPlay`, par
`teams/io/listeners/team_value_listener.rs`, qui réagit aux quatre événements
posant cette phase : `TeamEnrolled`, `DismissalsPhaseValidated`,
`MatchReportingCancelled`, `CostlyMistakesApplied`.

`GamePhaseOverridden` **n'en fait pas partie**. Un admin qui force une équipe en
`ReadyToPlay` la laisserait donc avec une TV périmée. Ajouter l'événement à
`ends_in_ready_to_play()` en implémentant cette carte — en filtrant sur
`to_phase == ReadyToPlay`, l'override pouvant viser n'importe quelle phase.
