SELECT rules::text AS rules
FROM   competitions
WHERE  id = $1
