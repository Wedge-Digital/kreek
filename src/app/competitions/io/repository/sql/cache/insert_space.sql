INSERT INTO competitions__space_cache (space_id, space_name, space_icon_path)
VALUES ($1, $2, $3)
ON CONFLICT (space_id) DO NOTHING