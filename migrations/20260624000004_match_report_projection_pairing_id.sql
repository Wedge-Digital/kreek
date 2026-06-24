ALTER TABLE match_report_projection ADD COLUMN IF NOT EXISTS pairing_id TEXT;

CREATE INDEX IF NOT EXISTS match_report_proj_pairing ON match_report_projection (pairing_id);
