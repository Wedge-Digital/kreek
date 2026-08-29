-- Les points de classement qui ne viennent d'aucun match : forfait, sanction,
-- rattrapage.
--
-- ── Une table à part de `ranking_lines`, et c'est ce qui les fait vivre ──────
-- Le rejeu (carte 418) recalcule le classement **depuis zéro** à partir des
-- cumuls de match. Tout ce qui vivrait dans `ranking_lines` serait donc effacé
-- au premier changement de barème — silencieusement, puisque le rejeu réussit.
--
-- ── `points` est signé ─────────────────────────────────────────────────────
-- Une pénalité est un point négatif, pas une autre nature de mouvement. Le
-- grand livre de trésorerie sépare montant et direction parce qu'il distingue
-- deux natures ; ici il n'y en a qu'une, et deux colonnes obligeraient à les
-- tenir cohérentes.
--
-- ── Ni `updated_at`, ni suppression logique ────────────────────────────────
-- Une ligne ne se modifie pas : elle se supprime et une autre la remplace. Un
-- `updated_at` laisserait croire le contraire. Un drapeau de suppression
-- compliquerait chaque lecture pour conserver une trace que personne n'a
-- demandée.
CREATE TABLE ranking__manual_points (
    id          BIGSERIAL PRIMARY KEY,
    season_id   TEXT NOT NULL,
    team_id     TEXT NOT NULL,
    points      INTEGER NOT NULL,
    reason      TEXT,
    awarded_by  TEXT NOT NULL,
    awarded_at  TIMESTAMPTZ NOT NULL DEFAULT now()
);

-- Deux index parce que les deux lectures diffèrent : le classement agrège par
-- saison, la page de gestion groupe par équipe.
CREATE INDEX ranking__manual_points_season_idx ON ranking__manual_points (season_id);
CREATE INDEX ranking__manual_points_season_team_idx ON ranking__manual_points (season_id, team_id);
