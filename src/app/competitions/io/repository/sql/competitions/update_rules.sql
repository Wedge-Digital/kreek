UPDATE competitions
SET    rules = $1::jsonb
WHERE  id    = $2
RETURNING id
