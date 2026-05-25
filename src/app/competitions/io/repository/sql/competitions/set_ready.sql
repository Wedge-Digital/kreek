UPDATE competitions
SET    status = 'ready'
WHERE  id = $1
RETURNING id