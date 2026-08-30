-- Le grand livre d'une équipe, chaque ligne accompagnée de l'événement qui l'a
-- produite (carte 435).
--
-- ── `LEFT JOIN` et non `JOIN` ──────────────────────────────────────────────
-- Une ligne dont l'événement manquerait doit s'afficher **sans détail** plutôt
-- que de disparaître : un relevé à trou se lit comme une erreur de calcul et se
-- cherche du mauvais côté, alors qu'un relevé sommaire se comprend.
--
-- ── `ORDER BY event_version`, jamais `occurred_at` ─────────────────────────
-- Deux mouvements d'un même traitement — l'achat de coups de pouce et son
-- remboursement, la recette et la bourde — partagent l'horodatage à la
-- milliseconde. Seule la version porte l'ordre dans lequel les soldes
-- s'enchaînent, et c'est cet enchaînement que le relevé rend visible.
--
-- ── L'index existe déjà ────────────────────────────────────────────────────
-- `teams__treasury_ledger_source`, la contrainte d'unicité
-- `(team_id, event_version)` qui sert le `ON CONFLICT` de l'écriture, couvre
-- exactement ce `WHERE` et cet `ORDER BY`. Vérifié à l'`EXPLAIN` plutôt que
-- supposé : `Index Scan using teams__treasury_ledger_source`, sans tri.
SELECT l.event_version,
       l.direction,
       l.amount_kpo,
       l.reason,
       l.balance_after_kpo,
       l.occurred_at,
       e.payload
FROM   teams__treasury_ledger l
LEFT JOIN team_event_store e
       ON e.team_id = l.team_id
      AND e.version = l.event_version
WHERE  l.team_id = $1
ORDER BY l.event_version
