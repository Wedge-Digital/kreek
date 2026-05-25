SELECT invitations::text AS invitations
FROM   competitions
WHERE  id = $1