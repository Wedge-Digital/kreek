-- Le nombre de blessures persistantes d'un joueur (carte 568).
--
-- Une blessure persistante naît d'une blessure sérieuse, et d'elle seule. Le
-- compteur vivait dans l'agrégat (`career_persistent_injuries`) ; la feuille
-- d'équipe le lit ici, sans rejouer l'event store.
--
-- **Recalculé, jamais incrémenté** : `recompute_persistent_injuries` le repose
-- depuis l'agrégat dans la transaction de l'événement, comme les `*_delta`. Une
-- dépublication de rapport le fait redescendre sans dire de combien.
--
-- Les joueurs existants sont remplis par la migration de données `m006`.
ALTER TABLE players_proj
    ADD COLUMN IF NOT EXISTS persistent_injuries SMALLINT NOT NULL DEFAULT 0;
