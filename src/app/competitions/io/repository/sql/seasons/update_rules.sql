UPDATE competition_seasons
SET    name   = $1,
       rules  = $2::jsonb,
       status = 'rules_selected'
WHERE  id     = $3
RETURNING id
