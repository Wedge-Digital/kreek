-- Brouillon de phase (carte 257).
--
-- Les phases de recrutement et de renvois fonctionnent au panier : le coach
-- accumule des lignes et les annule librement, rien n'est engagé avant la
-- validation de phase. Ce panier vit côté serveur, donc il se persiste.
--
-- Une seule table pour les deux phases, discriminée par `phase` : elles sont
-- séquentielles, les deux brouillons ne coexistent jamais.
--
-- `state` ne porte que **les lignes** du brouillon. Le catalogue du roster,
-- l'effectif et la trésorerie sont rechargés à chaque hydratation — c'est ce
-- qui garantit qu'un brouillon vieux de dix minutes est évalué contre les prix
-- et l'effectif d'aujourd'hui, et non contre ceux de sa création.
CREATE TABLE IF NOT EXISTS teams__phase_drafts (
    team_id    TEXT        NOT NULL,
    phase      TEXT        NOT NULL,
    space_id   TEXT        NOT NULL,
    state      JSONB       NOT NULL,
    version    INT         NOT NULL DEFAULT 1,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (team_id, phase)
);
