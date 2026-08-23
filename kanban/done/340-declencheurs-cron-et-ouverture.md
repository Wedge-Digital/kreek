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

- [x] `use_cases/send_due_notifications_use_case.rs` : commande portant `today`
      (**pas** une lecture d'horloge — c'est ce qui le rend testable)
- [x] `SendDueNotificationsReport` : saisons examinées, dues, envoyées, déjà
      envoyées, échouées
- [x] `src/cli/send_notifications.rs` + variante `SendNotifications` de `Command`
- [x] `--date` et `--dry-run` ; journalisation bruyante si `--date` ≠ aujourd'hui
- [x] `exit(1)` si `failed > 0` ou sur erreur de base
- [x] `use_cases/send_registration_open_use_case.rs`
- [x] `io/app_events/competition_ready_listener.rs`, `init(event_bus: …)`
- [x] E2E : terminer le magicien → `competition_notification_deliveries` porte
      une ligne par coach invité *(via `db_helpers.query_db`)*
- [x] `tests/impact-map.toml` mis à jour dans le même commit
- [x] Ligne de crontab documentée dans le README du dépôt
- [x] `make check-arch`, `make test` et `make e2e`

## Ce qui a été fait

Les trois requêtes de sélection **n'existaient pas** : la 335 n'avait écrit que
`claim` et `confirm`. Elles sont bornées par la date, donc le coût du cron reste
indépendant du nombre de saisons historiques, et celle de la date limite exclut
`= ''` autant que `IS NULL` — le champ du magicien rend la chaîne vide quand on
l'efface.

Le câblage du cron est monté dans la sous-commande plutôt que tiré d'`AppState` :
il n'a besoin ni du routeur, ni des sessions, ni des données de référence.

## Le défaut que seul un lancement à la main a montré

Les trois requêtes cherchent une journée **à la date donnée** ; `due_today()`
compare, elle, à `today + 1`, `+2`, `+3`. Je passais `today` aux trois. Le cron
ne trouvait donc **jamais rien** — `seasons=0` — sans la moindre erreur pour le
signaler.

Aucun test ne l'aurait vu : l'e2e de cette carte couvre l'ouverture, qui ne
passe pas par ces requêtes, et les tests unitaires du domaine ne connaissent pas
le SQL.

La correction ne se contente pas de décaler les dates à l'appel : une fonction
de domaine, `fenetres(today)`, rend les trois, **depuis les mêmes constantes que
`due_today()`**. Les recalculer côté use case aurait remis deux sources en
présence, et le symptôme serait revenu au premier changement de décalage. Trois
tests l'accompagnent, dont un qui vérifie qu'une journée trouvée par la fenêtre
de la veille est bien celle que `due_today()` annonce.

## Un test e2e vide, rattrapé

Le second scénario — « republier n'annonce pas deux fois » — passait alors que
le journal était **vide des deux côtés** : `0 == 0`. Il porte désormais une
assertion préalable qui refuse de conclure sans première annonce.

La cause du vide valait aussi d'être comprise : `create_full_competition` crée la
compétition en mode `invitation` **sans inviter personne**, donc zéro
destinataire. Le code avait raison, le jeu d'essai avait tort. Le constructeur
partagé accepte maintenant `access_mode`, avec son défaut inchangé.

## La chaîne complète, sur données réelles

| Passage | Résultat |
|---|---|
| `--dry-run` sur une veille de journée | 149 saisons, 149 dues, **0 envoi, 0 ligne** |
| 1ᵉʳ vrai passage | **550 lignes**, toutes confirmées |
| 2ᵉ passage, même date | `sent=0`, **`skipped=550`**, aucune ligne de plus |

Et l'ouverture : publier une compétition écrit une ligne `registration_open` par
membre, sans journée — le cas que l'index protège par `COALESCE(round_id, '')`.

## Ce qui reste ouvert

La **carte 338** attend sa vérification en client réel, désormais possible :
c'est cette carte qui l'a débloquée.
