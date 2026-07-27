-- Une équipe ne peut avoir qu'une seule ligne de classement par match.
--
-- La table n'avait que `id` en clé primaire : rien n'empêchait d'insérer deux
-- fois les lignes d'un même match, ce qui aurait doublé les points de l'équipe
-- sans laisser de trace. Le rejeu qui suit une correction de rapport rend ce
-- risque concret — l'index le fait échouer bruyamment plutôt que silencieusement.
CREATE UNIQUE INDEX ranking_lines_match_team
    ON ranking_lines (match_report_id, team_id);
