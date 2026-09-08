-- La campagne d'une journée. R2 en garantit une seule, l'index unique le tient.
SELECT id, season_id, round_id, deadline, auto_remind, opened_at, close_le,
       exemptee, appariee
FROM competition_presence_surveys
WHERE round_id = $1
