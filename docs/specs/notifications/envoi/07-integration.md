# Phase 7 — Effets de bord : le service d'envoi

**Entrée** : `06-domaine.md`, validée. Conception, pas implémentation.

## Le point d'accroche de R11 est un listener, pas une tâche détachée

La phase 5 disait « appelé depuis la validation de l'étape 5, en tâche
détachée ». L'investigation montre mieux : `execute_finalize` **émet déjà**
`CompetitionsDomainEvent::CompetitionReady` sur le bus interne du BC.

Un listener y souscrit, dans `io/app_events/competition_ready_listener.rs`.
Trois avantages sur la tâche détachée lancée depuis le handler :

- le handler reste un **traducteur HTTP pur**, comme l'exige le CLAUDE.md — il
  ne sait pas qu'un email part ;
- le détachement est acquis par le bus, sans `tokio::spawn` dans un handler ;
- c'est le pattern déjà en place dans ce BC (`match_report_*_listener.rs`).

**Convention de nommage à respecter** : `init(event_bus: &EventBus, …)` — sans
préfixe `app_`, puisqu'il s'agit du bus **interne**. `scripts/check-arch.sh` lit
cette signature pour l'axe 5. Ce listener n'écrivant aucune projection, l'axe ne
le vise pas ; la convention reste due par cohérence.

## Persistance

### La migration

```sql
-- migrations/<ts>_competition_notification_deliveries.sql

CREATE TABLE IF NOT EXISTS competition_notification_deliveries (
    notification_type TEXT        NOT NULL,
    season_id         TEXT        NOT NULL,
    round_id          TEXT,                 -- NULL : notification de saison
    target_date       TEXT        NOT NULL, -- la date visée, cf. R2
    coach_id          TEXT        NOT NULL,
    claimed_at        TIMESTAMPTZ NOT NULL DEFAULT now(),
    sent_at           TIMESTAMPTZ           -- NULL = réservé, non confirmé
);

-- Index unique sur COALESCE, et non contrainte UNIQUE ordinaire.
--
-- PostgreSQL ne considère jamais deux NULL comme égaux : une UNIQUE portant sur
-- round_id laisserait passer autant de doublons qu'on veut pour les deux
-- notifications de saison, qui n'ont pas de journée. La protection tomberait
-- exactement là où on la croit acquise, et seulement pour deux des quatre
-- notifications — donc de façon invisible en test si l'on ne teste que les
-- journées.
CREATE UNIQUE INDEX IF NOT EXISTS idx_notification_deliveries_key
    ON competition_notification_deliveries
       (notification_type, season_id, COALESCE(round_id, ''), target_date, coach_id);
```

### Les requêtes

| Fichier | Rôle |
|---|---|
| `list_seasons_with_round_starting.sql` | saisons ayant une journée non `rest` démarrant à la date donnée |
| `list_seasons_with_round_closing.sql` | saisons ayant une journée `time_frame` clôturant à la date donnée |
| `list_seasons_with_deadline.sql` | saisons dont `invitations->>'registration_deadline'` vaut la date donnée |
| `claim_delivery.sql` | `INSERT … ON CONFLICT DO NOTHING RETURNING 1` |
| `confirm_delivery.sql` | `UPDATE … SET sent_at = now()` |

Les trois premières sont **bornées par la date**, jamais parcourues en entier :
le coût du cron reste indépendant du nombre de saisons historiques.

`claim_delivery.sql` renvoyant zéro ligne signifie « déjà envoyé » — c'est la
base qui tranche, pas le code, et c'est tout R3.

La troisième interroge le JSONB des invitations. La chaîne vide y étant possible
(cf. phase 6), la requête doit exclure `= ''` autant que `IS NULL`.

## Les quatre gabarits

`assets/templates/emails/fr_FR/`, où vit déjà `lost_login.html` — et où `en_EN`
a été supprimé (français seul).

| Gabarit | Issu de la maquette |
|---|---|
| `competition_registration_open.html` | `invitation-competition.html` |
| `competition_round_eve.html` | `email-journee-demain.html` |
| `competition_round_closing.html` | `email-fin-de-journee.html` |
| `competition_registration_deadline.html` | `email-date-limite-inscription.html` |

Les maquettes portent leurs variantes **en commentaires HTML** ; la conversion
les transforme en `{% if %}` et `{% match %}`. Pour `competition_round_eve`,
**deux axes indépendants** (phase 4) : `date_end` pour la ligne de clôture,
`participation` pour le bloc des matchs. Quatre combinaisons, toutes
atteignables.

**Contraintes d'email, pas de page web** — le logo en `{{app_url}}/static/img/…`
et jamais en `data:` URI que Gmail retire ; `width` et `height` en **attributs
HTML**, Outlook ignorant le CSS de dimension ; aucun style qui dépende d'une
feuille externe. Ce sont les mêmes points que la carte 325 liste pour
`lost_login.html`.

`app_url` porte son schéma, depuis la configuration (phase 4) — sans recopier le
`http://` en dur de `send_reset_password_email`.

## La sous-commande CLI

```
kreek send-notifications [--date YYYY-MM-DD] [--dry-run]
```

`clap` kebab-case les variantes : `Serve` → `serve`, `SeedE2e` → `seed-e2e`,
donc `SendNotifications` → `send-notifications`.

| Argument | Rôle |
|---|---|
| *(aucun)* | la date du jour, dans le fuseau du serveur (R10) |
| `--date` | force la date visée — **indispensable pour tester sans attendre le lendemain**, et pour rejouer une journée à la main |
| `--dry-run` | résout et rend, n'envoie pas, ne journalise pas |

**`--date` mérite un mot.** Il permet à un opérateur de viser une date passée, ce
que R9 interdit au cron. Ce n'est pas une contradiction : R9 vise le comportement
**automatique**, pas une action explicite d'exploitant. La commande journalise
bruyamment quand la date fournie n'est pas celle du jour, pour qu'un `--date`
resté dans une crontab se voie.

Sortie : le rapport de la phase 5 — saisons examinées, notifications dues,
envoyées, déjà envoyées, échouées. **Code de sortie `1` si `failed > 0`** ou sur
erreur de base : c'est ce qui rend R1 observable dans les logs du cron. Une
exécution parfaite et une exécution ayant perdu douze emails ne doivent pas se
ressembler.

L'appel par le cron système, à documenter dans le README du dépôt :

```cron
# Une fois par jour, après minuit dans le fuseau du serveur.
30 6 * * *  cd /srv/kreek && EXEC_PROFILE=prod ./kreek send-notifications >> /var/log/kreek-notifications.log 2>&1
```

`main()` chargeant configuration, pool **et migrations** avant de dispatcher, la
commande ne peut pas tourner sur un schéma périmé. `compose(cfg, pool)` lui donne
tout le câblage sans dupliquer `main.rs`.

## Injection

`IEmailService` est déjà construit dans `main.rs` (`build_email_service`) et
injecté dans `AuthContext`. Il faut l'injecter de même dans
`CompetitionsContext`, ainsi que `app_url`.

`ICompetitionSpaceMemberPort` gagne `list_space_members` ; son adapter existant
s'appuie sur `list_members_for_space.sql`, déjà écrite dans `spaces`.

## Tests

### Unitaires — phase 6

`due_today()` et `DeliveryKey`, sans base ni réseau.

### Intégration — c'est ici que porte l'essentiel

`#[sqlx::test]`, vraie base, avec un **`IEmailService` espion** capturant les
envois au lieu de les expédier.

| Test | Ce qu'il garde |
|---|---|
| deux exécutions le même jour → **un seul** email par coach | R3, et l'index sur `COALESCE` |
| deux exécutions pour une notification **sans journée** → un seul email | le piège du `NULL` — ce test échouerait avec une `UNIQUE` ordinaire |
| un coach d'un autre espace inscrit de force → **pas d'email** | R7 |
| l'envoi échoue → ligne présente, `sent_at` à `NULL`, `failed = 1` | R1 |
| exécution du lendemain → la ligne à `NULL` **n'est pas rejouée** | R9, le cas le plus contre-intuitif |
| journée décalée d'un jour → un **second** envoi part | R2 |
| coach inscrit sans match → corps « tu ne joues pas » | R4 |
| coach avec deux équipes → **un** email listant **deux** matchs | la correction de la phase 5 |
| le HTML rendu contient l'adversaire, la journée et l'URL absolue | le rendu des gabarits |

Le deuxième test est celui qui n'aurait pas été écrit spontanément : en ne testant
l'idempotence que sur les notifications de journée, on validerait un index qui ne
protège pas les deux autres.

### E2E — un seul scénario, et la raison de sa solitude

**Le chemin de R11 est atteignable au navigateur** : terminer le magicien, puis
vérifier par `db_helpers.query_db` que `competition_notification_deliveries`
porte une ligne par coach invité. C'est un vrai test e2e, et il couvre le
listener, la résolution des destinataires et le journal.

**Le chemin du cron ne l'est pas.** C'est une commande CLI : aucun navigateur ne
la déclenche, et Playwright n'a rien à piloter.

Le CLAUDE.md exige unitaire **et** e2e pour toute fonctionnalité livrée. La
raison qu'il en donne est explicite — l'e2e existe parce que le rendu
HTML/HTMX/Alpine échappe aux tests unitaires. **Cette raison ne s'applique pas à
une commande sans interface** : il n'y a ni swap, ni événement DOM, ni composant
Alpine à casser.

L'équivalent de garantie est donc porté par les tests d'intégration ci-dessus :
vraie base, vrai schéma, vraies requêtes, service d'email espionné. C'est une
**dérogation nommée**, avec sa justification — pas un test sauté en silence.

Reste une chose que ni l'un ni l'autre ne voit : **le rendu visuel des quatre
emails dans un vrai client de messagerie**. Les maquettes ont été validées à
l'œil en phase 1 ; la conversion en Askama doit l'être de même, en s'envoyant les
quatre emails avec `EMAIL__PROVIDER=resend` sur une adresse de test. À faire à
l'implémentation, et à cocher dans la carte.

### Carte d'impact

`tests/impact-map.toml` doit recevoir l'entrée du nouveau test e2e **dans le même
commit** : l'axe 8 de `check-arch` est bloquant, et un test sans entrée est
traité comme `"all"` puis signalé.

## Ce que cette phase laisse à la suivante

La phase 8 — le découpage en cartes des **deux** unités, en une fois, ordonnées
de sorte que rien ne soit livrable tant que la chaîne n'est pas complète.
