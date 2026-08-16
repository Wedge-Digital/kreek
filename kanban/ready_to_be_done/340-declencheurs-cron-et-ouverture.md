# Les deux déclencheurs — CLI du cron et listener d'ouverture

**Spec :** `docs/specs/notifications/envoi/07-integration.md`, et R11
**Dépend de :** 336, 339
**Dernière carte de la chaîne**

> **C'est la carte qui allume la fonctionnalité.** Avant elle, aucun email ne
> part. Les neuf précédentes n'ont de valeur que livrées avec celle-ci.

## Objectif

Brancher les deux déclencheurs sur le cœur d'expédition.

## Conception

### Déclencheur 1 — la sous-commande CLI

```
kreek send-notifications [--date YYYY-MM-DD] [--dry-run]
```

`main()` charge configuration, pool **et migrations** avant de dispatcher : la
commande ne peut pas tourner sur un schéma périmé. `compose(cfg, pool)` donne
tout le câblage sans dupliquer `main.rs`.

**`--date` mérite un mot.** Il permet de viser une date passée, ce que R9
interdit au cron. Ce n'est pas une contradiction : R9 vise le comportement
**automatique**, pas une action explicite d'exploitant. La commande journalise
bruyamment quand la date fournie n'est pas celle du jour, pour qu'un `--date`
resté dans une crontab se voie.

**Code de sortie `1` si `failed > 0`.** C'est ce qui rend R1 observable : une
exécution parfaite et une exécution ayant perdu douze emails ne doivent pas se
ressembler.

### Déclencheur 2 — le listener d'ouverture

`execute_finalize` **émet déjà** `CompetitionsDomainEvent::CompetitionReady` sur
le bus interne. Un listener y souscrit — pas un `tokio::spawn` dans le handler :
celui-ci reste un traducteur HTTP pur, et le détachement est acquis par le bus.

Convention : `init(event_bus: &EventBus, …)`, **sans** préfixe `app_` — c'est le
bus interne, et `check-arch` lit cette signature.

## Checklist

- [ ] `use_cases/send_due_notifications_use_case.rs` : commande portant `today`
      (**pas** une lecture d'horloge — c'est ce qui le rend testable)
- [ ] `SendDueNotificationsReport` : saisons examinées, dues, envoyées, déjà
      envoyées, échouées
- [ ] `src/cli/send_notifications.rs` + variante `SendNotifications` de `Command`
- [ ] `--date` et `--dry-run` ; journalisation bruyante si `--date` ≠ aujourd'hui
- [ ] `exit(1)` si `failed > 0` ou sur erreur de base
- [ ] `use_cases/send_registration_open_use_case.rs`
- [ ] `io/app_events/competition_ready_listener.rs`, `init(event_bus: …)`
- [ ] E2E : terminer le magicien → `competition_notification_deliveries` porte
      une ligne par coach invité *(via `db_helpers.query_db`)*
- [ ] `tests/impact-map.toml` mis à jour dans le même commit
- [ ] Ligne de crontab documentée dans le README du dépôt
- [ ] `make check-arch`, `make test` et `make e2e`
