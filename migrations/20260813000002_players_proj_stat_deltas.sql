-- Cumul des ajustements de caractéristiques, toutes sources confondues
-- (carte 303).
--
-- La projection ne portait jusqu'ici **aucune** caractéristique : tout était
-- résolu à la lecture par `player_stats_service::resolve_stats`, qui compose la
-- base du poste (BC `references`) avec les séquelles et les augmentations SPP.
--
-- Ces colonnes portent des **deltas**, jamais des valeurs absolues. C'est ce
-- choix qui permet de les écrire sans interroger `references` : un cumul de
-- deltas ne dépend que des événements du joueur. Une valeur absolue aurait exigé
-- la base du poste, donc un port applicatif dans la transaction d'append.
--
-- Signées : une séquelle ou une dégradation par customisation les rend négatives.
ALTER TABLE players_proj
    ADD COLUMN IF NOT EXISTS ma_delta SMALLINT NOT NULL DEFAULT 0,
    ADD COLUMN IF NOT EXISTS st_delta SMALLINT NOT NULL DEFAULT 0,
    ADD COLUMN IF NOT EXISTS ag_delta SMALLINT NOT NULL DEFAULT 0,
    ADD COLUMN IF NOT EXISTS pa_delta SMALLINT NOT NULL DEFAULT 0,
    ADD COLUMN IF NOT EXISTS av_delta SMALLINT NOT NULL DEFAULT 0;
