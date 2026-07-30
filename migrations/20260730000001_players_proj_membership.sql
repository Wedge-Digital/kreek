-- Appartenance à l'effectif (carte 260).
--
-- Axe distinct de `participation_status`, qui décrit ce qu'un match a fait au
-- joueur. L'appartenance répond à une décision de coach : ce joueur est-il
-- encore de l'équipe ? C'est elle qui décide si le joueur figure dans les
-- lectures d'effectif.
--
-- Aucun joueur n'a jamais été renvoyé — la fonctionnalité naît avec cette
-- série. Le défaut suffit, il n'y a pas de reprise de données à faire.
ALTER TABLE players_proj
    ADD COLUMN IF NOT EXISTS membership TEXT NOT NULL DEFAULT 'Active';

-- Toutes les lectures d'effectif filtrent désormais sur cette colonne, et
-- toutes portent déjà `team_id` : l'index composite sert les sept chemins.
CREATE INDEX IF NOT EXISTS idx_players_proj_team_membership
    ON players_proj (team_id, membership);
