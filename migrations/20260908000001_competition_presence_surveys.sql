-- La campagne de présence et ses réponses — épic E16, carte 510.
--
-- Ce que la base garantit ici, le code ne l'arbitre pas : les trois index
-- uniques portent R1, R2 et l'unicité du jeton, et le CHECK interdit les
-- combinaisons que le type Rust ne sait pas exprimer.

CREATE TABLE competition_presence_surveys (
    id          TEXT PRIMARY KEY,
    season_id   TEXT        NOT NULL,
    round_id    TEXT        NOT NULL,
    deadline    TEXT        NOT NULL,
    auto_remind BOOLEAN     NOT NULL,
    opened_at   TEXT        NOT NULL,

    -- `Fermeture::Decidee { le }`. Il n'y a **pas** de colonne `statut` : R23 le
    -- calcule depuis l'échéance et cette date. Une colonne aurait pu dire
    -- « ouverte » sur une campagne échue, et rien n'aurait signalé la
    -- divergence — la seule façon de ne pas avoir à synchroniser deux vérités
    -- est de n'en stocker qu'une.
    close_le    TEXT,

    -- `Appariement::Fait { exemptee }`. Les rencontres, elles, appartiennent à
    -- la journée (R24) : les recopier ici créerait une seconde vérité qu'aucune
    -- transaction ne tient avec la première.
    exemptee    TEXT,
    appariee    BOOLEAN     NOT NULL DEFAULT FALSE,

    created_at  TIMESTAMPTZ NOT NULL DEFAULT now(),

    -- R2 — un second sondage ouvert sur la même journée produirait deux jeux de
    -- jetons vivants et deux vérités sur qui est présent.
    CONSTRAINT competition_presence_surveys_round_unique UNIQUE (round_id)
);

CREATE INDEX idx_presence_surveys_season ON competition_presence_surveys (season_id);

CREATE TABLE competition_presence_answers (
    id              TEXT PRIMARY KEY,
    survey_id       TEXT NOT NULL
                    REFERENCES competition_presence_surveys (id) ON DELETE CASCADE,
    team_id         TEXT NOT NULL,
    coach_id        TEXT NOT NULL,

    -- Le jeton vit ici et non dans une table à part : R7 lui refuse toute
    -- échéance propre, il n'a donc rien à porter qu'une réponse ne porte déjà.
    token           TEXT NOT NULL,

    presence        TEXT NOT NULL
                    CHECK (presence IN ('sans_reponse', 'presente', 'absente')),
    repondu_le      TEXT,

    -- R6 — renseigné quand l'organisateur a saisi à la place du coach, ce que
    -- l'écran signale par « saisi par vous ». NULL couvre aussi bien le jeton
    -- que l'encart : le canal est un fait d'autorisation, vivant le temps de
    -- l'écriture, pas une propriété de la réponse (R28).
    saisi_par_admin TEXT,

    -- R1 — une réponse par équipe engagée, jamais par coach.
    CONSTRAINT competition_presence_answers_team_unique UNIQUE (survey_id, team_id),
    CONSTRAINT competition_presence_answers_token_unique UNIQUE (token),

    -- PostgreSQL n'a pas de type somme : ces trois colonnes sont la projection
    -- à plat de `Presence`, et ce CHECK est le seul endroit où l'impossibilité
    -- d'une réponse déclarée sans horodatage s'écrit côté base.
    CONSTRAINT competition_presence_answers_coherentes CHECK (
        (presence = 'sans_reponse' AND repondu_le IS NULL AND saisi_par_admin IS NULL)
        OR (presence <> 'sans_reponse' AND repondu_le IS NOT NULL)
    )
);

CREATE INDEX idx_presence_answers_survey ON competition_presence_answers (survey_id);
