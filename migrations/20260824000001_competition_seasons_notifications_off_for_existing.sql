-- R8 pour les saisons antérieures — carte 366, épic E02.
--
-- La migration 20260822000001 a ajouté la colonne sans remplir les lignes
-- existantes, en renvoyant la décision « avant la carte 340 ». La 340 est
-- passée sans que personne ne la reprenne : 318 saisons sur 471 avaient
-- `notifications` à `NULL`, donc lues comme « les quatre allumées » par le
-- défaut serde. La spec exige l'inverse pour elles :
--
--   « Les saisons existantes démarrent éteintes, les nouvelles allumées. »
--   docs/specs/notifications/README.md — R8
--
-- Éteindre plutôt qu'assumer : les deux anciens interrupteurs
-- (`use_mail_notification`, `notify_by_email`) ne branchaient rien, personne
-- n'a jamais reçu un seul de ces e-mails, et certaines de ces saisons sont
-- terminées. Les honorer rétroactivement enverrait des messages que plus
-- personne n'attend.
UPDATE competition_seasons
SET    notifications = '{"registration_open":false,
                         "round_eve":false,
                         "round_closing":false,
                         "registration_deadline":false}'::jsonb
WHERE  notifications IS NULL;

-- Le défaut « saison neuve » — les quatre allumées, seconde moitié de R8.
--
-- Il est indispensable et non décoratif : `insert_season.sql` n'écrit que
-- `(id, competition_id, name)`. Sans ce DEFAULT, le NOT NULL ci-dessous ferait
-- échouer toute création de saison. L'étape 4 du magicien réécrit ensuite la
-- colonne avec les choix de l'organisateur.
ALTER TABLE competition_seasons
    ALTER COLUMN notifications SET DEFAULT '{"registration_open":true,
                                             "round_eve":true,
                                             "round_closing":true,
                                             "registration_deadline":true}'::jsonb;

-- Ce qui fait la différence entre une correction et un verrou.
--
-- Sans cette contrainte, un futur INSERT oubliant la colonne recreuserait le
-- trou en silence — et le test de sérialisation continuerait de passer, comme
-- il l'a fait pendant que 318 saisons devenaient notifiantes.
ALTER TABLE competition_seasons
    ALTER COLUMN notifications SET NOT NULL;
