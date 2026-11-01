-- Qui a saisi chaque action du journal (carte 635).
--
-- Écrite à l'insertion depuis `ActionRecorded.recorded_by`, dans la
-- transaction de l'événement, et remplie ici pour l'existant depuis l'event
-- store : une projection reconstruite donne donc ce que donne cette migration.
--
-- **L'event store n'est pas touché.** `recorded_by` est obligatoire dans
-- `ActionRecorded` depuis sa création (`d1b0d820`), et `ActionId` comme
-- `CoachId` se sérialisent en chaîne : `->>` rend la valeur telle quelle.
--
-- **Nullable** : une action sans événement correspondant resterait sans auteur
-- plutôt que d'en recevoir un inventé. La carte mesure sur un dump qu'il n'y en
-- a aucune.
ALTER TABLE match_report_actions
    ADD COLUMN IF NOT EXISTS recorded_by TEXT;

UPDATE match_report_actions a
   SET recorded_by = e.payload->>'recorded_by'
  FROM match_report_event_store e
 WHERE e.match_report_id = a.match_report_id
   AND e.event_type = 'ActionRecorded'
   AND e.payload->>'action_id' = a.action_id
   AND a.recorded_by IS NULL;
