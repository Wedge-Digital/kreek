-- Les matchs d'une équipe, sur une saison (carte 477).
--
-- Le SELECT est celui de `list_resultats.sql` **au caractère près** : c'est ce
-- qui garantit que le même `MatchResultatVm` se construit sans adaptation.
--
-- Aucun filtre de statut, contrairement aux résultats : les trois sont voulus,
-- « mon prochain match » étant ce qu'un coach vient chercher ici.
--
-- L'ordre n'est pas celui de la compétition, et les deux tris opposés d'un même
-- CASE se lisent mal — d'où ce commentaire :
--
--   1. le match en cours de saisie, s'il y en a un
--   2. les matchs à venir, du plus proche au plus lointain  (position ASC)
--   3. les matchs joués, du plus récent au plus ancien       (position DESC)
--
-- Reprendre le `round_position DESC` des résultats en incluant les matchs à
-- venir mettrait le plus lointain en tête et enterrerait le prochain au milieu.
--
-- Ni curseur ni LIMIT : une saison compte des centaines de matchs, une équipe
-- en joue dix à quinze.
SELECT
    pairing_id, round_id, round_name, round_position,
    round_date_start, round_date_end, round_day_type,
    home_team_id, home_team_name, home_roster_name, home_coach_name, home_logo_url, home_initials,
    away_team_id, away_team_name, away_roster_name, away_coach_name, away_logo_url, away_initials,
    match_status, home_score, away_score, home_casualties, away_casualties, match_report_url
FROM competition_match_display_proj
WHERE season_id = $1
  AND (home_team_id = $2 OR away_team_id = $2)
ORDER BY
    CASE match_status WHEN 'in_progress' THEN 0 WHEN 'upcoming' THEN 1 ELSE 2 END,
    CASE WHEN match_status = 'upcoming' THEN round_position END ASC,
    round_position DESC
