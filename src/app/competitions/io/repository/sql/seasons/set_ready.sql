UPDATE competition_seasons
SET    status = 'ready'
WHERE  id     = $1
RETURNING id