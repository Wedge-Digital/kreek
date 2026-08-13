-- Ordre libre du joueur dans l'effectif, posé par glisser-déposer (carte 291).
--
-- Nullable à dessein : un joueur jamais réordonné n'a pas de rang, et le tri
-- retombe alors sur son numéro de maillot. Une valeur par défaut aurait obligé
-- à inventer un ordre initial pour tout l'existant — or « pas encore rangé »
-- et « rangé en première position » ne sont pas le même fait.
--
-- Aucun joueur n'a jamais été réordonné, la fonctionnalité naissant avec cette
-- série : il n'y a pas de reprise de données à faire.
ALTER TABLE players_proj
    ADD COLUMN IF NOT EXISTS display_order INTEGER;
