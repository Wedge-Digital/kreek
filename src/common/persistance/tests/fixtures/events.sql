INSERT INTO event_log (event_id, emitter, event_type, tags, payload, occurred_at)
VALUES
    ('01JQQQQQQQQQQQQQQQQQQQQA01', '01JQQQQQQQQQQQQQQQQQQQQ001', 'TeamDraftCreated', '{"team_id": "team-abc"}', '{"name": "Les Bleus"}',  '2026-01-01 10:00:00+00'),
    ('01JQQQQQQQQQQQQQQQQQQQQA02', '01JQQQQQQQQQQQQQQQQQQQQ001', 'RulesetSelected',  '{"team_id": "team-abc"}', '{"ruleset": "BB2020"}', '2026-01-01 11:00:00+00'),
    ('01JQQQQQQQQQQQQQQQQQQQQA03', '01JQQQQQQQQQQQQQQQQQQQQ001', 'RosterSelected',   '{"team_id": "team-abc"}', '{"roster": "Humains"}',  '2026-01-01 12:00:00+00'),
    ('01JQQQQQQQQQQQQQQQQQQQQA04', '01JQQQQQQQQQQQQQQQQQQQQ001', 'TeamDraftCreated', '{"team_id": "team-xyz"}', '{"name": "Les Rouges"}', '2026-01-01 09:00:00+00');