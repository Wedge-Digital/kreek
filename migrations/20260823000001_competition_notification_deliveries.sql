-- Journal des envois de notification (carte 335, épic E02).
--
-- La ligne est insérée **avant** l'envoi : c'est elle qui réserve le créneau, et
-- deux crons parallèles se disputent l'index. `sent_at` n'est renseigné qu'après
-- confirmation, donc une ligne restée à `NULL` est un **échec constaté** — la
-- journalisation que R1 demande. Elle n'est jamais rejouée le lendemain : la
-- reprendre serait le « cherche ce qui n'est pas parti » que R9 interdit.
--
-- `target_date` est la date **visée**, pas la date d'envoi. C'est ce seul choix
-- qui fait tenir R2 : une journée décalée change la clé, donc réarme la
-- notification, sans qu'une ligne de code lui soit consacrée.
CREATE TABLE IF NOT EXISTS competition_notification_deliveries (
    notification_type TEXT        NOT NULL,
    season_id         TEXT        NOT NULL,
    round_id          TEXT,                 -- NULL : notification de saison
    target_date       TEXT        NOT NULL, -- la date visée, cf. R2
    coach_id          TEXT        NOT NULL,
    claimed_at        TIMESTAMPTZ NOT NULL DEFAULT now(),
    sent_at           TIMESTAMPTZ           -- NULL = réservé, non confirmé
);

-- Index unique sur `COALESCE`, et **non** une contrainte `UNIQUE` ordinaire.
--
-- PostgreSQL ne considère jamais deux `NULL` comme égaux : une `UNIQUE` portant
-- sur `round_id` laisserait passer autant de doublons qu'on veut pour les deux
-- notifications de saison, qui n'ont pas de journée. La protection tomberait
-- exactement là où on la croit acquise, et seulement pour deux des quatre
-- notifications — donc de façon invisible en test si l'on ne teste que les
-- journées.
CREATE UNIQUE INDEX IF NOT EXISTS idx_notification_deliveries_key
    ON competition_notification_deliveries
       (notification_type, season_id, COALESCE(round_id, ''), target_date, coach_id);
