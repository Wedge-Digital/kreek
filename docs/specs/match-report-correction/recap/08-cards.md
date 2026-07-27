# Phase 8 — Cartes kanban · page Recap

11 cartes, plus la carte de bug 225 identifiée en phase 6.

## Liste ordonnée

| # | Carte | Dépend de | Résumé |
|---|---|---|---|
| 225 | `225-bug-disponibilite-joueur-blesse` | — | Bug préexistant : un joueur blessé pendant un match semble remis disponible aussitôt. Fixe la définition de « avant » pour la règle 15 |
| 226 | `226-mrc-autorisation-publication` | — | Dette n°1 : `is_authorized()` absent du POST de publication |
| 227 | `227-mrc-domaine-depublication` | — | VOs d'éligibilité, événement, `unpublish()`, arête `rehydrate`, drapeau `was_published_before` |
| 228 | `228-mrc-garde-fou-ports` | 227 | Les 2 méthodes de port, leurs adapters, le domain service d'éligibilité |
| 229 | `229-mrc-use-case-handler` | 226, 228 | Use case, handler POST, route |
| 230 | `230-mrc-zone-correction-recap` | 228 | VM, builder, template, CSS, bandeau |
| 231 | `231-mrc-publisher-app-events` | 229 | Payloads et émission des 3 app events de compensation |
| 232 | `232-mrc-compensation-competitions` | 231 | Projection résultats/calendrier remise en `in_progress` |
| 233 | `233-mrc-compensation-ranking` | 231 | Index unique, `delete_lines_for_match`, use case, listener |
| 234 | `234-mrc-compensation-teams` | 231 | Instantané dérivé, `revert_post_match_sequence`, projection |
| 235 | `235-mrc-compensation-players` | 231, **225** | Instantané dérivé, `revert_match_impact`, projection |
| 236 | `236-mrc-e2e-correction` | 232–235 | Les 8 scénarios E2E |

## Chemin critique

```
227 ──► 228 ──┬──► 229 ──► 231 ──┬──► 232, 233, 234 ──┐
              │                   │                    ├──► 236
226 ──────────┘                   └──► 235 ◄── 225 ────┘
```

La carte 225 est la seule dépendance externe au périmètre de la feature, et elle
ne bloque que la 235.

## Choix de découpage

**Une carte par BC pour les compensations**, pas une carte unique. Chacune est
indépendamment compilable et testable, chacune touche un BC différent, et les
cartes 234 et 235 sont substantielles à elles seules (état dérivé + méthode
domaine + projection). Les regrouper donnerait une carte impossible à finir en
une session.

**L'index unique sur `ranking_lines` est replié dans la carte 233** plutôt que
d'être une carte prérequis à part : la migration est minuscule et la carte
`ranking` est déjà propriétaire de cette table. L'ordre reste satisfait.

**Les cartes 229 et 230 sont séparées** bien qu'elles touchent le même
contrôleur : la 230 (l'affichage) ne dépend que du garde-fou, pas du use case.
Elles peuvent être menées dans l'ordre inverse si l'on préfère voir l'écran
avant que l'action ne fonctionne.

## Points de vigilance transverses

Trois pièges reviennent dans plusieurs cartes et méritent d'être en tête avant
de commencer :

1. **Les projections** (cartes 234, 235) — un événement de compensation non
   traité par la fonction de projection **compile sans broncher** et laisse
   l'affichage figé sur les valeurs post-match, avec un agrégat pourtant juste.
   D'où l'exigence de tests d'intégration repository et pas seulement unitaires.
2. **Le publisher** (carte 231) — sa relecture exige aujourd'hui l'état
   `Published` et logue un `warn!` sinon. Après une dépublication il trouvera
   `ReadyToPublish` : sans adaptation, **aucune compensation ne se déclenche**,
   en silence.
3. **La non-inversibilité du clamp des fans** (carte 234) — `clamp(0, 20)` n'est
   pas injectif. La restauration se fait par instantané, jamais par
   soustraction. Les deux tests correspondants sont les tests décisifs de la
   feature.
