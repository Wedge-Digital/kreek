-- Panier de customisation d'un joueur (carte 305).
--
-- Même nature que `teams__phase_baskets` : une table de **travail**, pas un
-- flux d'événements. Le commissaire empile des lignes et les retire librement ;
-- rien n'est engagé avant la validation, qui seule produit des événements.
--
-- Clé `player_id` seul : le panier est propre au **joueur**, pas à son auteur.
-- Deux commissaires le partageraient donc — cas écarté, sa probabilité métier
-- étant jugée quasi nulle, et le validateur endossant alors tout le panier.
--
-- `state` ne porte **que les lignes**. Le joueur, le catalogue de compétences et
-- les caractéristiques de base sont rechargés à chaque hydratation : c'est ce
-- qui garantit qu'un panier d'une heure est jugé contre le joueur
-- d'aujourd'hui, et non contre celui de sa création.
--
-- `updated_at` porte la péremption : un panier vit 24 h après sa **dernière
-- modification**, pas après sa création — c'est l'inactivité qui périme, pas
-- l'ancienneté. La vérification se fait à l'ouverture de la fiche, sans tâche
-- planifiée : le panier y est de toute façon chargé pour décider du mode.
CREATE TABLE IF NOT EXISTS players__customisation_baskets (
    player_id  TEXT        PRIMARY KEY,
    space_id   TEXT        NOT NULL,
    state      JSONB       NOT NULL,
    version    INT         NOT NULL DEFAULT 1,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
