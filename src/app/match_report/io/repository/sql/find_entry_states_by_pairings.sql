-- L'état de saisie des rapports rattachés à des appariements (carte 676).
-- Un rapport annulé n'est plus rattaché à son appariement, comme dans
-- `find_id_by_pairing`.
SELECT pairing_id, match_report_id, competition_id, home_team_id, away_team_id,
       phase, entry_mode, started_by
FROM match_report_proj
WHERE space_id = $1
  AND pairing_id = ANY($2)
  AND phase != 'Cancelled'
