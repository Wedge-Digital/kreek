SELECT c.name  AS competition_name,
       c.logo,
       cm.coach_id,
       uc.coach_name
FROM   competitions c
LEFT JOIN competitions_members cm ON cm.competition_id = c.id
LEFT JOIN spaces__user_cache uc ON uc.id = cm.coach_id
WHERE  c.id = $1
ORDER BY uc.coach_name
