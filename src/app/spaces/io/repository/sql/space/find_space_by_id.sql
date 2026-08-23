-- Les coachs viennent de `spaces__user_cache`, dépôt souverain de ce BC. Cette
-- requête lisait auparavant la table des comptes du BC d'authentification : un
-- BC n'interroge pas les tables d'un autre, et `spaces` est extractible — il
-- doit pouvoir être copié sans emporter son voisin.
--
-- `coach_icon` est nullable. C'est `us.coach_id` qui dit si la ligne porte un
-- membre, jamais l'icône : confondre les deux fait disparaître de l'agrégat
-- tout coach sans avatar.
SELECT
    s.id              AS space_id,
    s.space_name,
    s.space_icon_path,
    us.coach_id       AS coach_id,
    u.coach_name,
    u.coach_icon,
    us.profile
FROM spaces s
LEFT JOIN spaces__user_space us ON us.space_id = s.id
LEFT JOIN spaces__user_cache u  ON u.id = us.coach_id
WHERE s.id = $1
