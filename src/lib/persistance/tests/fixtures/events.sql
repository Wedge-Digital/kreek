INSERT INTO event_log (id, source, "type", spec_version, time, data_schema, data_content_type, subject, data)
VALUES
    ('evt-001', '/team-creation', 'TeamDraftCreated', '1.0', '2026-01-01 10:00:00+00', '/schemas/team', 'application/json', 'team-abc', '{"name":"Les Bleus"}'),
    ('evt-002', '/team-creation', 'RulesetSelected',  '1.0', '2026-01-01 11:00:00+00', '/schemas/team', 'application/json', 'team-abc', '{"ruleset":"BB2020"}'),
    ('evt-003', '/team-creation', 'RosterSelected',   '1.0', '2026-01-01 12:00:00+00', '/schemas/team', 'application/json', 'team-abc', '{"roster":"Humains"}'),
    ('evt-004', '/team-creation', 'TeamDraftCreated', '1.0', '2026-01-01 09:00:00+00', '/schemas/team', 'application/json', 'team-xyz', '{"name":"Les Rouges"}');