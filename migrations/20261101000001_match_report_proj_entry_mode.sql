-- Le mode de saisie d'un compte rendu et celui qui l'a démarré (carte 631).
--
-- Projetés depuis le flux dans la transaction de l'événement, et par le même
-- pli que l'agrégat : `SelectionConfirmed` pose « Je saisis tout » et son
-- `confirmed_by` comme démarreur, `EntryModeChosen` les remplace. Une
-- projection reconstruite depuis l'event store donne donc ce que donne cette
-- migration.
--
-- **L'event store n'est pas touché** : aucun `EntryModeChosen` n'est écrit pour
-- l'existant, le rejeu d'un flux sans lui retombe sur la même valeur.
--
-- Tout compte rendu confirmé passe en « Je saisis tout », démarré par celui qui
-- a confirmé. Un brouillon jamais confirmé garde `started_by` vide : il n'est
-- pas démarré.
ALTER TABLE match_report_proj
    ADD COLUMN IF NOT EXISTS entry_mode TEXT NOT NULL DEFAULT 'SoloEntry',
    ADD COLUMN IF NOT EXISTS started_by TEXT;

UPDATE match_report_proj p
   SET started_by = e.payload->>'confirmed_by'
  FROM match_report_event_store e
 WHERE e.match_report_id = p.match_report_id
   AND e.event_type = 'SelectionConfirmed'
   AND p.started_by IS NULL;
