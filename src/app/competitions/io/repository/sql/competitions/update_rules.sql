UPDATE competitions
SET    rules  = $1::jsonb,
       status = 'rules_selected'
WHERE  id     = $2
RETURNING id
