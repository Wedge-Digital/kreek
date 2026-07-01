-- Renommage des tables de projection vers la convention _proj.
-- Cette migration gère les bases de données déjà provisionnées avant ce changement.
-- Les bases créées à partir de zéro (make init_db) appliquent directement les nouveaux noms.

ALTER TABLE IF EXISTS players_projection        RENAME TO players_proj;
ALTER TABLE IF EXISTS team_enrollment_projection RENAME TO team_enrollment_proj;
ALTER TABLE IF EXISTS team_projection            RENAME TO team_proj;
ALTER TABLE IF EXISTS match_report_projection    RENAME TO match_report_proj;

-- Renommage des index impactés

ALTER INDEX IF EXISTS players_projection_team_id    RENAME TO players_proj_team_id;
ALTER INDEX IF EXISTS idx_team_enrollment_season_status RENAME TO idx_team_enrollment_proj_season_status;
ALTER INDEX IF EXISTS idx_team_projection_season_status RENAME TO idx_team_proj_season_status;
