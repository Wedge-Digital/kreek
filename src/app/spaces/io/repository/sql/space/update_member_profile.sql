-- `space_id` **et** `coach_id` : la clé de `spaces__user_space` est composite.
-- L'oublier toucherait le même coach dans tous ses espaces, sans erreur — et
-- passerait tout test qui n'utilise qu'un seul espace.
UPDATE spaces__user_space
SET    profile = $3
WHERE  space_id = $1 AND coach_id = $2
