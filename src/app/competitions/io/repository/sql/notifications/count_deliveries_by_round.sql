-- Ce que le journal sait dire d'une journée : combien sont partis, combien non.
--
-- **`count(*)` compte les créneaux réservés, `count(sent_at)` ceux qui sont
-- attestés partis.** La différence est le nombre d'échecs constatés — une ligne
-- réservée dont l'envoi a échoué reste à `sent_at NULL`, et c'est précisément ce
-- que R20 veut journaliser.
--
-- Aucun compteur n'est stocké à côté : la vérité est ici, et l'écran la relit.
-- Persister « 12 envoyés » sur la campagne créerait une seconde vérité à tenir
-- d'accord avec celle-ci, pour économiser une requête sur un panneau qu'on ouvre
-- à la main.
--
-- **Groupé par `target_date`, et c'est ce qui sépare les envois.** L'ouverture
-- porte l'échéance de la campagne ; chaque relance porte son propre jour d'envoi
-- (cf. `io/email/survey_mailer.rs`). Un groupement par type seul aurait
-- additionné toutes les relances en un nombre que personne ne sait lire.
--
-- Ordonné par date : la dernière ligne d'un type est son envoi le plus récent.
SELECT notification_type,
       target_date,
       count(*)        AS reserves,
       count(sent_at)  AS attestes
FROM competition_notification_deliveries
WHERE season_id = $1
  AND round_id = $2
GROUP BY notification_type, target_date
ORDER BY target_date
