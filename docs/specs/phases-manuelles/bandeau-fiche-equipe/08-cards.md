# Bandeau de la fiche équipe · Phase 8 : les cartes

**Entrée** : `07-integration.md` validé.

## Les cinq cartes

| # | Intitulé | Ce qu'elle livre | Ce qu'on peut constater à la fin |
|---|---|---|---|
| **575** | Le domaine sait ouvrir une phase manuelle | `PhaseEntry`, `OverridablePhase`, `OverrideReason`, quatre événements, `open_phase_override`, les sorties, la garde de `revert_post_match_sequence`, `returns_to_ready_to_play()` | `cargo test` au vert sur les règles de la phase 6 — aucun écran |
| **576** | La projection et les listeners suivent | `team_proj` exhaustive (bras d'avance compris), listeners sur `returns_to_ready_to_play()`, adapter de correction | une sortie manuelle recalcule la TV et purge les paniers |
| **577** | L'ouverture s'écrit | commande, use case | une ouverture en base, refusée hors de « prête à jouer » |
| **578** | Le panneau d'ouverture du bandeau | VMs, route, contrôleur, gabarits, Alpine, CSS | un admin ouvre une phase depuis la fiche |
| **579** | Les tests e2e des phases manuelles | six scénarios Playwright | les gardes tiennent au-delà de l'écran |

## Ce qui commande l'ordre

**575 d'abord, et seule** : elle casse volontairement la compilation —
`to_app_event()` et `type_name()` sans joker, `returns_to_ready_to_play()`
exhaustif. Ces ruptures sont le mécanisme de sûreté du BC.

**576 avant le reste** : sans elle, une phase ouverte n'apparaîtrait pas en
projection, et « Mes équipes » contredirait la fiche — un faux défaut pour la
578.

**577 avant 578** : l'écran s'appuie sur une écriture déjà testée.

**579 en dernier, pas en option** : ses scénarios de refus serveur prouvent que
la garde ne repose pas sur le bouton masqué.

## Ce qui part avec

- La carte **46** passe en `cancelled/`, remplacée par celles-ci.
- Pas d'épic : une seule page, cinq cartes, listées dans `kanban/epics/README.md`.
- **Aucune migration** : `phase_entry` se reconstruit au rejeu, `team_proj` garde
  ses colonnes.
