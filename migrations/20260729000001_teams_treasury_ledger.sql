-- Grand livre de trésorerie et tag dérivé (carte 255).
--
-- La trésorerie était déjà dérivée des événements, mais son historique n'était
-- consultable nulle part : on connaissait le solde, jamais son chemin.

-- Tag posé à l'append depuis `Team::treasury_movement()`, jamais à la main.
-- Même forme que `event_log`, index GIN compris.
ALTER TABLE team_event_store
    ADD COLUMN IF NOT EXISTS tags JSONB NOT NULL DEFAULT '[]'::jsonb;

CREATE INDEX IF NOT EXISTS idx_team_event_store_tags
    ON team_event_store USING GIN (tags);

-- Une ligne par mouvement. `amount_kpo` est le montant **effectif** : une
-- bourde de 50 kPo sur une caisse de 30 en retire 30. `balance_after_kpo` vient
-- du domaine, jamais d'un calcul SQL — sans quoi la règle d'écrêtage à zéro
-- existerait à deux endroits.
CREATE TABLE IF NOT EXISTS teams__treasury_ledger (
    id                BIGSERIAL   PRIMARY KEY,
    team_id           TEXT        NOT NULL,
    event_version     BIGINT      NOT NULL,
    direction         TEXT        NOT NULL,
    amount_kpo        INT         NOT NULL,
    reason            TEXT        NOT NULL,
    balance_after_kpo INT         NOT NULL,
    occurred_at       TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX IF NOT EXISTS teams__treasury_ledger_team
    ON teams__treasury_ledger (team_id, id);

-- Rend l'alimentation idempotente : rejouer un événement ne duplique pas sa ligne.
CREATE UNIQUE INDEX IF NOT EXISTS teams__treasury_ledger_source
    ON teams__treasury_ledger (team_id, event_version);
