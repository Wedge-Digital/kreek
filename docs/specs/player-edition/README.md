# Player edition — Spec index

Édition de l'effectif directement depuis la page de détail d'équipe : renommer
un joueur, changer son numéro de maillot, réordonner librement les lignes du
tableau — sans quitter la page. Maquetté dans
`assets/rawpages/html/app-team-detail.html` (Phase 1 validée), disponible
uniquement quand l'équipe est dans l'état « Prête à jouer ».

Contexte : aujourd'hui `personal_name` est toujours vide en base
(`player_repository.rs`, écrit en dur `""` à la création du joueur) et
`jersey` est figé au recrutement — aucun événement domaine ne permet de les
modifier après coup. L'ordre d'affichage actuel
(`ORDER BY jersey NULLS LAST, player_id`) n'a pas de notion d'ordre
indépendante du numéro de maillot.

## Pages

| Page | Front | Back | DTOs | Use cases | Domaine | Intégration | Cartes |
|---|---|---|---|---|---|---|---|
| team-detail | ✅ | ✅ | ✅ | | | | |

## Règles métier transverses (identifiées phase 1)

- **Disponible uniquement en état « Prête à jouer »** — le bouton « Modifier
  l'effectif » n'existe que dans ce bandeau ; changer d'état pendant l'édition
  annule proprement (cf. maquette, `setTeamState`).
- **Cross-BC** : le déclencheur (bandeau d'état, page hôte du BC `teams`) et
  le tableau à éditer (widget `players-widget`, BC `players`) ne partagent pas
  de BC — communication exclusivement par événements DOM sur `body`.
- **Périmètre joueurs** : seuls les joueurs `membership = 'Active'` sont
  éditables — pas les renvoyés (`Dismissed`) ni les joueurs en retraite
  temporaire.
- **Pas d'état d'erreur maquetté** : le cas de conflit serveur (doublon
  détecté en concurrence) est décrit en texte dans `02-front.md`, pas
  maquetté visuellement — jugé rare, décision prise en Phase 2.
