-- Le solde de SPP d'un joueur — ce qu'il peut encore dépenser (carte 569).
--
-- `spp` porte le cumul des gains : un achat n'en retire rien. Le solde vivait
-- dans l'agrégat seul (`Player::spp_remaining`), et la feuille d'équipe
-- rejouait chaque joueur pour l'afficher. Il est désormais projeté.
--
-- **Recalculé, jamais incrémenté** : `recompute_spp_remaining` le repose
-- depuis l'agrégat dans la transaction de l'événement, comme les `*_delta` et
-- `persistent_injuries`.
--
-- Les joueurs existants sont remplis par la migration de données `m007`.
ALTER TABLE players_proj
    ADD COLUMN IF NOT EXISTS spp_remaining INTEGER NOT NULL DEFAULT 0;
