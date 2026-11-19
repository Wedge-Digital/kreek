# K19 — Grilling

> Reconstitué à la migration (2026-11-19), à partir de l'épic `kanban/epics/done/E19-saisie-du-compte-rendu-droits-et-robustesse.md` (devenue `initiative.md`). Le dépôt ne garde
> pas de trace d'un grilling : les besoins ci-dessous reprennent l'objectif et les cartes, à relire.

## Questions et réponses

Aucune trace dans le dépôt.

## Besoins

- **K19-B1** — Terminé quand (critère de l'épic E19) : **Constaté à la clôture (2026-11-01)** : le 403 d'un tiers et « publié par » le sont en e2e (`test_match_report_access.py`, `test_match_report_recap.py`). Le 409 d'un conflit ne l'est **qu'en tests unitaires** — aucun e2e ne provoque deux écritures entrelacées, et le rechargement par `HX-Refresh` n'a pas été observé dans un navigateur. Clôture acceptée avec cette réserve. Un membre de l'espace qui n'est ni l'un des deux coachs ni admin reçoit un 403 sur toute écriture d'un compte rendu, constaté par un test e2e. Deux écritures simultanées sur le même compte rendu donnent un 409 et un message, jamais un 500. Le récapitulatif de tout compte rendu publié — y compris ceux publiés avant la livraison — nomme le coach qui l'a publié.
- **K19-B2** — `match_report` — `MatchReportConfirmed` doit naître d'un domain event (ticket `352-match-report-confirmed-passe-par-le-publisher`)
- **K19-B3** — Le camp du coach courant (ticket `621-le-camp-du-coach-courant`)
- **K19-B4** — Les routes du compte rendu sont gardées (ticket `622-les-routes-du-compte-rendu-sont-gardees`)
- **K19-B5** — Un conflit d'écriture se dit (ticket `623-un-conflit-d-ecriture-se-dit`)
- **K19-B6** — « Publié par » au récapitulatif (ticket `624-publie-par-au-recapitulatif`)

## Hors périmètre

Voir la section « Ce que l'épic ne couvre pas » de `initiative.md`, s'il y en a une.
