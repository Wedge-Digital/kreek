-- Compteurs cumulés servant aux critères de départage. Accumulés pour tous les
-- critères indépendamment de leur activation (règle 12) : le calcul reste
-- découplé de la configuration et la projection rejouable.
--
-- Pas de colonne `diff_td` : c'est une valeur dérivée (td_for − td_against),
-- calculée à la comparaison (règle 13).
--
-- DEFAULT 0 : les lignes déjà écrites n'ont pas cette information. Aucun backfill
-- — la projection est rebuildable depuis l'event store. Conséquence en
-- développement : deux équipes à égalité de points sur des lignes antérieures
-- seront ex æquo sur tous les critères, quels que soient les matchs joués.
ALTER TABLE ranking_lines
    ADD COLUMN td_for      INTEGER NOT NULL DEFAULT 0,
    ADD COLUMN td_against  INTEGER NOT NULL DEFAULT 0,
    ADD COLUMN casualties  INTEGER NOT NULL DEFAULT 0,
    ADD COLUMN fouls       INTEGER NOT NULL DEFAULT 0,
    ADD COLUMN completions INTEGER NOT NULL DEFAULT 0;
