-- Points bonus cumulés, conservés à part du total de `ranking_points` (qui les
-- contient déjà). Alimente l'onglet « Classement détaillé », où chaque nombre
-- composant le total doit être vérifiable.
--
-- DEFAULT 0 : les lignes déjà écrites n'ont pas cette information. Aucun backfill
-- n'est prévu — la projection est rebuildable depuis l'event store.
ALTER TABLE ranking_lines ADD COLUMN bonus_points INTEGER NOT NULL DEFAULT 0;
