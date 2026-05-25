UPDATE competitions
SET    name = $1,
       logo = $2
WHERE  id   = $3
RETURNING id