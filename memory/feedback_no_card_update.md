---
name: feedback-no-card-update
description: Ne jamais modifier une carte kanban déjà réalisée (dans done/)
metadata:
  type: feedback
---

Ne jamais mettre à jour une carte kanban déjà en `done/`. C'est formellement interdit.

**Why:** Les cartes terminées sont des artefacts immuables — elles documentent ce qui a été fait. Les modifier brouillerait l'historique et la traçabilité.

**How to apply:** Si une réalisation passée doit être étendue ou corrigée, créer une **nouvelle carte** avec une dépendance explicite sur la carte originale. Ne jamais rouvrir une carte en `done/`.
