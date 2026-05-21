SELECT EXISTS (
    SELECT 1 FROM competitions
    WHERE space_id = $1
      AND name    = $2
)