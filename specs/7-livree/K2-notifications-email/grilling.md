# K2 — Grilling

> Reconstitué à la migration (2026-11-19), à partir de l'épic `kanban/epics/done/E02-notifications-email.md` (devenue `initiative.md`). Le dépôt ne garde
> pas de trace d'un grilling : les besoins ci-dessous reprennent l'objectif et les cartes, à relire.

## Questions et réponses

Aucune trace dans le dépôt.

## Besoins

- **K2-B1** — Terminé quand (critère de l'épic E02) : Un coach inscrit à une compétition reçoit, la veille d'une journée, un e-mail listant ses matchs — sans que personne n'ait lancé de commande à la main, et sans qu'une seconde exécution du cron le même jour lui en envoie un second. **Constaté** par `tests/e2e/test_notification_cron.py`, qui construit deux équipes inscrites, programme une journée au lendemain, invoque le binaire, et lit la table du journal. Les trois tests échouent si la journée est décalée d'un jour — vérifié, et c'est ce qui distingue un critère atteint d'un critère supposé. La 338 a complété ce constat par ce qu'aucun test ne peut dire : ce que ces e-mails **donnent à voir**. Les trois notifications du cron ont été rendues par la commande de production elle-même, et le premier rendu a immédiatement montré un `<img src="http:///…">` — `AppConfig::app_url()` transforme un domaine vide en `"http://"` au lieu de refuser, et `.env.dev` comme `.env.remote.demo` l'ont vide. **L'ouverture dans Outlook et Gmail n'a pas eu lieu** : elle demandait un envoi réel depuis une clé Resend, et le sujet a été jugé maîtrisé. L'épic est close sans cette dernière observation, et il vaut mieux que ce soit écrit qu'oublié.
- **K2-B2** — L'email de mot de passe au standard visuel des emails Kreek (ticket `325-email-mot-de-passe-au-nouveau-standard`)
- **K2-B3** — Réglages de notification — domaine et persistance (ticket `331-notifications-domaine-et-persistance`)
- **K2-B4** — Widget de réglage des notifications — et son hôte admin (ticket `332-widget-reglage-notifications-admin`)
- **K2-B5** — Widget de réglage des notifications — dans le magicien (ticket `333-widget-reglage-notifications-magicien`)
- **K2-B6** — Retrait des trois réglages morts de l'étape 3 (ticket `334-retrait-des-trois-reglages-morts`)
- **K2-B7** — Journal d'envois — table et repository (ticket `335-journal-envois-table-et-repository`)
- **K2-B8** — Domaine de l'ordonnancement — `due_today()` (ticket `336-domaine-ordonnancement-due-today`)
- **K2-B9** — Résolution des destinataires (ticket `337-resolution-des-destinataires`)
- **K2-B10** — Les quatre gabarits d'email (ticket `338-gabarits-email-notifications`)
- **K2-B11** — Le cœur d'expédition (ticket `339-coeur-expedition-notifications`)
- **K2-B12** — Les deux déclencheurs — CLI du cron et listener d'ouverture (ticket `340-declencheurs-cron-et-ouverture`)
- **K2-B13** — R8 n'a jamais été tranchée, et 318 saisons sont notifiantes (ticket `430-saisons-existantes-allumees-contre-r8`)
- **K2-B14** — Le chemin du cron n'a aucun test, et il porte trois notifications sur quatre (ticket `431-le-chemin-du-cron-n-a-aucun-test`)
- **K2-B15** — R8 n'a jamais été tranchée, et 318 saisons sont notifiantes (ticket `590-saisons-existantes-allumees-contre-r8`)
- **K2-B16** — Le chemin du cron n'a aucun test, et il porte trois notifications sur quatre (ticket `591-le-chemin-du-cron-n-a-aucun-test`)

## Hors périmètre

Voir la section « Ce que l'épic ne couvre pas » de `initiative.md`, s'il y en a une.
