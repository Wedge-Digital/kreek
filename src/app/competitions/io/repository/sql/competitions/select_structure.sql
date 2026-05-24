SELECT structure::text AS structure
FROM   competitions
WHERE  id = $1
