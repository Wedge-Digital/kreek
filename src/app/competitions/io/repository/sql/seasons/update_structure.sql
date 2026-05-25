UPDATE competition_seasons
SET    structure = $1::jsonb,
       status    = 'structure_selected'
WHERE  id        = $2
RETURNING id
