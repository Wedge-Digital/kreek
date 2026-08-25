-- Le registre des migrations de **données**, distinctes des migrations de
-- schéma que sqlx tient dans `_sqlx_migrations`.
--
-- Pourquoi une seconde famille : certaines corrections ont besoin du corpus de
-- règles, qui vit hors du dépôt (`REFERENCES__DIR`) et n'est lu qu'au milieu du
-- démarrage. Une migration SQL ne peut donc savoir ni quelles compétences sont
-- Élite, ni quels rosters portent « Lineman a vil prix » : ces faits n'existent
-- que dans des fichiers JSON que la base ne voit pas.
--
-- Une ligne par migration appliquée. Le nom y est écrit **dans la transaction
-- de la migration elle-même** : la table protège du rejeu, elle ne protège pas
-- d'une interruption au milieu, et seule l'atomicité le fait.
CREATE TABLE applied_data_migrations (
    name       text PRIMARY KEY,
    applied_at timestamptz NOT NULL DEFAULT now()
);
