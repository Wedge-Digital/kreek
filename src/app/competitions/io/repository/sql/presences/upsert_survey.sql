-- La campagne. `created_at` n'est jamais réécrite : elle date la ligne, pas la
-- campagne, dont l'ouverture est portée par `opened_at`.
INSERT INTO competition_presence_surveys
    (id, season_id, round_id, deadline, auto_remind, opened_at, close_le, exemptee, appariee)
VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
ON CONFLICT (id) DO UPDATE SET
    deadline    = EXCLUDED.deadline,
    auto_remind = EXCLUDED.auto_remind,
    close_le    = EXCLUDED.close_le,
    exemptee    = EXCLUDED.exemptee,
    appariee    = EXCLUDED.appariee
