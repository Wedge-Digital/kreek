UPDATE competitions
SET    invitations = $1::jsonb,
       status      = 'invitations_configured'
WHERE  id          = $2
RETURNING id