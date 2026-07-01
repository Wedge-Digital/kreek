SELECT
    pairing_id, round_id, round_name, round_position,
    round_date_start, round_date_end, round_day_type,
    home_team_name, home_roster_name, home_coach_name, home_logo_url, home_initials,
    away_team_name, away_roster_name, away_coach_name, away_logo_url, away_initials,
    match_status, home_score, away_score, home_casualties, away_casualties, match_report_url
FROM competition_match_display_proj
WHERE season_id = $1
  AND match_status IN ('in_progress', 'completed')
  AND ($2::integer IS NULL OR round_position < $2)
ORDER BY round_position DESC
LIMIT 500
