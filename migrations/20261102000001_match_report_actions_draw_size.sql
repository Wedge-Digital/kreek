-- Le nombre de désignés d'un MVP tiré au sort (carte 638).
--
-- Posée par la projection de `MvpDrawn`, dans la transaction de l'événement
-- et sur l'action que l'`ActionRecorded` du même append vient d'insérer. Le
-- journal en tire « tiré parmi 6 désignés », et retire le ✕ : le tirage est
-- définitif.
--
-- **Nullable, et rien à remplir** : `MvpDrawn` naît avec cette carte, aucun
-- événement de l'event store ne le porte. Tout l'existant — actions de match,
-- MVP saisis à la main — reste à NULL, ce qui est juste.
ALTER TABLE match_report_actions
    ADD COLUMN IF NOT EXISTS draw_size SMALLINT;
