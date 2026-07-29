-- Renomme `teams__phase_drafts` en `teams__phase_baskets`.
--
-- « draft » portait déjà quatre sens dans ce dépôt — brouillon d'équipe, de
-- rapport de match, de compétition —, tous désignant une entité inachevée qui
-- **devient** l'entité réelle. Le panier de recrutement n'est pas ça : il
-- accumule des lignes, produit des événements, puis est supprimé. Il ne devient
-- rien.
--
-- La table est vide partout : la migration d'origine date du même jour et
-- aucune fonctionnalité ne l'écrit encore. C'est le dernier moment où ce
-- renommage ne coûte rien.
ALTER TABLE teams__phase_drafts RENAME TO teams__phase_baskets;
ALTER INDEX teams__phase_drafts_pkey RENAME TO teams__phase_baskets_pkey;
