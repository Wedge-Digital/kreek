---
name: feedback-kanban-columns
description: Règle de placement des cartes Kanban selon leur état de maturité
metadata:
  type: feedback
---

Placer les cartes Kanban dans le bon répertoire selon leur état :

- `kanban/ready_to_be_done/` — carte suffisamment raffinée pour être développée immédiatement (périmètre clair, checklist définie, dépendances identifiées). C'est là que vont les cartes qu'on vient de discuter et affiner en conversation.
- `kanban/to_be_refined/` — idée ou besoin à creuser, sans encore assez de détail pour démarrer (questions ouvertes, dépendances floues, scope incertain).
- `kanban/in_progress/` — carte en cours de développement.
- `kanban/done/` — carte terminée et livrée.

**Why:** l'utilisateur a corrigé plusieurs fois des cartes placées dans `to_be_refined` alors qu'elles venaient d'être affinées en conversation et étaient prêtes à développer.

**How to apply:** après avoir rédigé une carte suite à une discussion avec l'utilisateur, la placer dans `ready_to_be_done` par défaut sauf si des questions restent ouvertes — dans ce cas `to_be_refined`.