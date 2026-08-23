-- Même piège que la mise à jour du profil : la clé est composite, et omettre
-- `space_id` retirerait le coach de tous ses espaces d'un coup.
DELETE FROM spaces__user_space
WHERE  space_id = $1 AND coach_id = $2
