SELECT
    cmdp.pairing_id, cmdp.season_id, c.id AS competition_id, c.name AS competition_name,
    cmdp.round_name, cmdp.home_team_id, cmdp.home_team_name, cmdp.home_score,
    cmdp.away_team_id, cmdp.away_team_name, cmdp.away_score,
    cmdp.match_report_url, cmdp.published_at
FROM competition_match_display_proj cmdp
JOIN competition_seasons cs ON cs.id = cmdp.season_id
JOIN competitions c ON c.id = cs.competition_id
WHERE c.space_id = $1 AND cmdp.match_status = 'completed'
ORDER BY cmdp.published_at DESC NULLS LAST
LIMIT $2
