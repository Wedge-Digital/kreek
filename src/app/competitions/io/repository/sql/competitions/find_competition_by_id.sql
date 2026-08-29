-- Les administrateurs d'une compétition, pour `require_admin_access`.
--
-- ── Le filtre sur `competition_profile` vit dans le `ON` ────────────────────
-- C'est un `LEFT JOIN` : posée en `WHERE`, une condition sur la table jointe le
-- transformerait en jointure interne, et une compétition **sans membre**
-- cesserait d'être trouvée — `find_base_info` rendrait `None`, et tout ce qui en
-- dépend répondrait `404` sur une compétition qui existe.
--
-- ── Pourquoi ce filtre (carte 479) ─────────────────────────────────────────
-- Sans lui, `admin_ids` et `admin_names` portaient **tous** les membres, quel
-- que soit leur profil. Le nom des champs disait « admin », leur contenu disait
-- « membre », et `require_admin_access` s'y fiait : un participant inscrit avec
-- `CompetitionUser` serait devenu commissaire — libre de modifier le barème, de
-- retirer des poules, d'attribuer des points manuels.
--
-- Sans effet tant que rien n'écrit `CompetitionUser`, ce qui était le cas. Le
-- défaut ne se serait pas manifesté au moment de la faute : le code inscrivant
-- le participant aurait été correct, et c'est cette lecture-ci, écrite ailleurs
-- et plus tôt, qui l'aurait mal interprété.
SELECT c.name  AS competition_name,
       c.logo,
       cm.coach_id,
       uc.coach_name
FROM   competitions c
LEFT JOIN competitions_members cm
       ON cm.competition_id = c.id
      AND cm.competition_profile = 'CompetitionAdmin'
LEFT JOIN spaces__user_cache uc ON uc.id = cm.coach_id
WHERE  c.id = $1
ORDER BY uc.coach_name
