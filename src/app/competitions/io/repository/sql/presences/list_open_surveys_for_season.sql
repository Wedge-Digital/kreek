-- Les campagnes potentiellement ouvertes d'une saison, en une requête.
--
-- **Cette requête élague, elle ne décide pas.** R23 fait de la clôture un calcul
-- du domaine : `statut(maintenant)` croise l'échéance et la décision, et rien
-- n'écrit « close » en base. Le filtre ci-dessous n'est donc qu'un pré-tri, et
-- l'appelant repasse derrière.
--
-- Pourquoi élaguer quand même : l'encart s'affiche sur la page de détail d'une
-- compétition, que la plupart des visiteurs ouvrent sans être concernés. Charger
-- les vingt campagnes d'une saison pour en jeter dix-neuf ferait payer chacune de
-- ces visites.
--
-- **`>=` et non `>`, et la nuance n'est pas cosmétique.** `statut_de` ferme sur
-- `aujourd_hui > deadline` : une campagne échue le 10 octobre l'est à partir du
-- 11, donc elle répond encore le 10. Un `>` ici écarterait les campagnes du jour
-- même — et ce serait le seul sens dans lequel ce compromis est dangereux. Un
-- élagage trop large ne coûte qu'une campagne chargée pour rien, que `statut()`
-- écarte ; un élagage trop étroit fait **disparaître** une campagne ouverte, et
-- le domaine n'a plus rien à rattraper.
--
-- Les dates sont comparées en texte, comme partout dans ce BC : le format
-- `YYYY-MM-DD` est validé par `DateString` et `SurveyDeadline`, et son ordre
-- lexicographique est son ordre chronologique.
SELECT id, season_id, round_id, deadline, auto_remind, opened_at, close_le,
       exemptee, appariee
FROM competition_presence_surveys
WHERE season_id = $1
  AND close_le IS NULL
  AND deadline >= $2
ORDER BY round_id
