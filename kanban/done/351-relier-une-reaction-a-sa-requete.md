# `common` — Remonter d'une réaction à la requête qui l'a causée

**Priorité : basse** — confort de diagnostic, pas un manque bloquant
**Dépend de :** cartes 345, 348 et 350 (sans lignes des deux côtés, il n'y a
rien à relier)
**Fichiers :** `src/common/services/event_bus/domain_event_publication.rs`
(nouveau), les 21 sites d'émission sur bus interne, les sept publishers,
`scripts/check-arch.sh`

## Le problème

Un coach signale que son équipe est créée mais que ses joueurs n'apparaissent
pas. On part de son `x-request-id`, on déroule sa requête… et la piste s'arrête
net. Les réactions qu'elle a provoquées vivent sous d'autres identifiants, sans
lien avec elle.

Après les cartes 345, 348 et 350, on a **deux morceaux de piste et rien qui les
relie** :

```
grep rid=01M0AB
  ├─ req{rid=01M0AB path=/teams/submit coach=Bagouze}: → requête reçue
  ├─ req{rid=01M0AB …}: kreek::use_case: cmd=SubmitTeamCommand{…} duree_ms=12
  └─ req{rid=01M0AB …}: ← réponse envoyée status=200
                                    ⟂  la piste s'arrête ici
grep event_id=01M0ZZ
  ├─ app_event_publication{domain_event=TeamSubmitted}: event_id=01M0ZZ app event émis
  ├─ app_event{event_id=01M0ZZ}: … réaction de teams
  └─ app_event{event_id=01M0ZZ}: … réaction de players
```

**Rien dans le premier bloc ne mentionne un identifiant d'événement.** On ne
peut pas passer d'un `grep` à l'autre : on ne sait pas quoi chercher.

La coupure tombe là pour une raison mécanique : le publisher tourne dans **sa
propre tâche `tokio`**, lancée au démarrage. Quand il émet, il ignore tout de la
requête qui a mis l'événement sur le bus.

## Ce que le raffinage a changé

La version précédente de cette carte posait comme acquis qu'il fallait **faire
voyager le `rid`** jusqu'aux enveloppes, via un `tokio::task_local!` posé par la
couche web. Elle laissait ouverte la question « où le lire ? », et c'est ce qui
la maintenait à raffiner.

La question a trouvé une réponse, mais elle a surtout rendu la question
inutile : **il n'y a pas besoin de faire voyager quoi que ce soit.** Il suffit
de journaliser les identifiants aux deux points de passage, et d'accepter trois
`grep` au lieu d'un.

| | Faire voyager le `rid` | Journaliser les passages |
|---|---|---|
| Diagnostic | un `grep` | trois, chacun immédiat |
| Touche `EventEnvelope` | oui, champ nouveau | non |
| État ambiant | oui — posé par le web, lu ailleurs | non |
| Fichiers touchés | ~30 | ~10 |
| Mode de panne | un ambiant oublié à une frontière de tâche donne un `rid` faux **ou absent, sans que rien ne le signale** | aucun : chaque ligne ne dit que ce qu'elle sait |

Le dernier point a emporté la décision. Cette épic a passé quatre cartes à
débusquer des mécanismes qui **ressemblent à du travail fait sans rien
livrer** — une cible hors filtre, un span sans événement, un identifiant
régénéré en route. Payer un état ambiant traversant tout le système, au mode de
panne silencieux, pour économiser deux `grep` sur une carte que sa propre
en-tête classe « confort de diagnostic », est un mauvais échange.

## Trois faits établis au raffinage

**Le domaine construit l'enveloppe mais ne l'émet jamais.** Les 21 émissions de
production vivent dans `use_cases/` (16) et `io/` (5), **zéro dans `domain/`**.
C'est ce qui rend le point d'accroche disponible sans toucher au domaine.

Le premier inventaire en annonçait 25 : il excluait les chemins `tests/` mais
pas les modules `#[cfg(test)]` en fin de fichier, où quatre sites fabriquent des
événements pour amorcer un pipeline. L'axe 13 exempte les deux formes.

**`tags` n'est pas mort** — la version précédente de cette carte l'affirmait, et
proposait d'y loger la causalité ou de le supprimer. C'est vrai des app events
et **faux des domain events** : `team_creation`, `auth`, `spaces` et
`competitions` le remplissent via `get_tags()`, et un test de `team_repository`
relit `["treasury"]` depuis `team_event_store`. La question « le supprimer ? »
est close : non.

**L'enveloppe n'est jamais sérialisée en bloc.** `event_log` et les event stores
écrivent colonne par colonne. Ce fait ne sert plus à cette carte, qui ne touche
plus l'enveloppe — il est noté ici parce qu'il condamnait une inquiétude
(« ajouter un champ casse les replays ») qui n'avait pas lieu d'être.

## Ce qu'il faut faire

### 1. Une ligne à l'émission d'un domain event

Un `emettre(bus, enveloppe)` dans `common/services/event_bus/`, symétrique
exact du `publier()` de la carte 350 :

```rust
tracing::info!(
    event = %enveloppe.event_type,
    event_id = %enveloppe.event_id,
    "domain event émis"
);
```

Appelé depuis la tâche de la requête, il hérite du span `req` : **sa ligne porte
le `rid` et l'identifiant de l'événement émis.** C'est le premier chaînon
manquant.

**Au niveau `info` et non `debug`.** Le filtre de production vaut `info` : une
ligne posée en dessous n'existe pas là où on en a besoin. C'est la leçon des
cartes 344 et 349, et elle vaut d'être répétée parce qu'elle s'est déjà
présentée deux fois sous deux formes différentes.

### 2. Un champ `cause` sur le span du publisher

Les sept publishers ouvrent déjà, depuis la carte 350, un span
`app_event_publication{domain_event=…}`. Il gagne un champ :

```rust
cause = %envelope.event_id
```

La ligne d'émission de l'app event porte alors **l'identifiant reçu et
l'identifiant produit sur la même ligne**. C'est le second chaînon — et c'est
exactement là qu'il doit être, puisque le publisher est le seul endroit du
système qui voit les deux.

### 3. Rien à faire côté listeners

Ils portent déjà `event` et `event_id` depuis la carte 345.

## La chaîne, une fois faite

```
grep rid=01M0AB       →  … cmd=SubmitTeamCommand{…}
                         … event=TeamSubmitted event_id=01M0WW domain event émis
grep 01M0WW           →  app_event_publication{domain_event=TeamSubmitted cause=01M0WW}:
                         event=TeamCreated event_id=01M0ZZ app event émis
grep 01M0ZZ           →  app_event{event=TeamCreated event_id=01M0ZZ}: … teams
                         app_event{event=TeamCreated event_id=01M0ZZ}: … players
```

Trois sauts, chacun immédiat, et chaque ligne ne dit que ce qu'elle sait
réellement.

## Le verrou

`check-arch` **axe 13**, bloquant : aucun `.send(` sur un bus hors de
`emettre()` et `publier()`. Même raison que l'axe 12 — sans lui, la prochaine
émission ajoutée sera muette et personne ne le saura. Les tests sont exemptés,
et le fichier des deux helpers aussi.

## Suite

L'axe 13 posé ici a été fusionné dans l'axe 12 par la carte 355 : chercher les
noms `bus` et `event_bus` laissait passer tout récepteur nommé autrement. Le
numéro 13 sert désormais à vérifier que les cibles de journalisation relèvent de
`kreek::`.

## Ce que la carte ne fait pas

**Elle ne fait toujours pas voyager d'identifiant de causalité.** Si le besoin
d'un `grep` unique se manifeste réellement — c'est-à-dire si les trois sauts se
révèlent pénibles à l'usage, ce qui reste à constater — la voie de l'ambiant
reste ouverte et rien de ce qui est fait ici ne sera à défaire.

**Elle ne persiste rien.** L'audit — « qui a modifié quelle équipe, quand » —
est hors périmètre de l'épic : l'event store le donne déjà.

## Trouvés en chemin, hors périmètre

- **Le champ `tags` a quatre formes différentes** → **carte 357**, qui a trouvé
  plus grave : il est écrit par quatre chemins et **lu par personne**,
  `find_by_tag()` n'ayant aucun appelant.
- **L'axe 2 est plus étroit que la règle qu'il tient** → **carte 356**, versée
  dans l'épic E04. Son `grep` n'interdit au domaine que
  `axum|sqlx|tower|askama`, alors que `CLAUDE.md` lui interdit tout appel
  async — `tokio` n'y figure pas. La version précédente de cette carte
  proposait de lire un `task_local` depuis `domain/` : le verrou l'aurait
  laissée passer.

## Checklist

- [x] `emettre()` journalise `event` et `event_id` au niveau `info`, et les
      21 sites d'émission de production passent par lui
- [x] Les sept spans de publisher portent `cause = %envelope.event_id`
- [x] Axe 13 `check-arch` bloquant, **vérifié sur un cas volontairement
      fautif** — et le premier essai de vérification était vide, la
      substitution de test n'ayant pas pris : un axe qu'on croit éprouvé sur un
      cas qui n'en est pas un ne vaut pas mieux que l'axe 11 à sa première
      écriture
- [x] Test : `emettre()` journalise l'identifiant de l'enveloppe envoyée, et
      l'enveloppe part bien sur le bus
- [x] Vérifié en conditions réelles sur une création d'équipe : les trois
      `grep` enchaînés mènent du `x-request-id` aux réactions de `teams` et
      `players`
- [x] `make lint`, `make test` et `make check-arch` passent
