# `app` — Le versant émission des app events entre dans le journal

**Priorité : moyenne**
**Dépend de :** carte 345 (les listeners sont instrumentés — celle-ci fait la
symétrie), carte 348 (tant que les use cases sont muets, la moitié de ce
qu'elle relie n'existe pas)
**Fichiers :** les sept `src/app/*/io/app_events/app_event_publisher.rs`,
`src/common/services/event_bus/app_event_publication.rs`,
`scripts/check-arch.sh`

## Le problème

Les 19 listeners ouvrent un span portant `event` et `event_id`. Le versant
**émission**, lui, était muet — ou plutôt bavard sans contexte, sur le seul
publisher qui journalisait quoi que ce soit :

```
INFO kreek::app::team_creation::io::app_events::app_event_publisher:
     team_creation_app_event_publisher: relaying TeamSubmitted to app bus
```

Cette ligne ne porte **aucun identifiant**. On savait qu'un événement avait été
relayé, pas lequel — impossible de la relier aux spans des listeners qui vont y
réagir.

Le piège est double, et il apparaît en clair sur cet exemple : **le nom change
en route.** `TeamSubmitted` est le nom du domain event ; l'app event qui en
résulte s'appelle `TeamCreated`. C'est conforme à la règle de nommage de
`CLAUDE.md` — un domain event dit ce qui s'est passé dans son domaine, sans
trahir sa destination — mais à la lecture du journal, rien ne dit que ces deux
noms désignent le même fait. On cherche « TeamCreated » dans les logs et on ne
trouve pas l'émission.

## Le point à ne pas manquer

`to_enveloppe()` **engendre un nouvel identifiant** :

```rust
EventEnvelope {
    event_id: EventId::new().to_string(),
    …
}
```

L'app event n'a donc pas l'identifiant du domain event dont il est issu. Pour
que la ligne d'émission soit reliable aux spans des listeners, c'est
**l'identifiant de l'enveloppe produite** qu'il faut journaliser — pas celui
reçu sur le bus interne. Une carte qui journaliserait l'identifiant d'entrée
produirait une trace qui a l'air correcte et ne corrèle rien.

C'est la raison d'être de `publier(bus, enveloppe)` : **la fonction ne voit que
l'enveloppe produite.** Le piège est fermé par construction, pas par
discipline — onze lignes recopiées à la main au-dessus de onze `send` auraient
eu toutes les chances d'en rater une.

## Ce que la carte n'avait pas vu

**Six publishers sur sept ne journalisaient rien.** La carte disait « remplacer
la ligne actuelle » ; cette ligne n'existait que dans `team_creation`, celui
qu'elle citait en exemple.

**`match_report` n'a pas la structure des six autres.** La carte annonçait
« les sept publishers partagent la même structure, la modification est
mécanique ». En réalité il publie depuis **cinq sites**, dans des fonctions
appelées à deux ou trois niveaux de profondeur — sur onze sites d'émission au
total, cinq sont chez lui.

C'est ce qui a fait choisir un **span** pour porter `domain_event`, là où la
carte disait « ils ne prennent pas de span, une ligne suffit ». L'alternative
était de faire descendre un `&str` à travers cinq signatures pour les besoins
du journal — décorer l'appelant, ce que l'épic a écarté pour les use cases. Le
span est aussi ce que font déjà les 19 listeners : les deux versants se lisent
pareil.

**Le nom du domain event était déjà là, gratuitement.** L'enveloppe reçue sur
le bus interne porte `event_type`, qui est exactement ce nom. Rien à extraire.

## La violation trouvée en chemin

Deux use cases émettent des app events **directement** :
`create_match_report_use_case` et `update_match_selection_use_case`, tous deux
pour `MatchReportConfirmed`. `CLAUDE.md` l'interdit — un app event doit naître
d'un domain event, converti par le publisher — et la raison du court-circuit
est visible : le publisher de `match_report` ne traite pas `MatchReportConfirmed`.

Ils passent par `publier` pour au moins entrer dans le journal. **Le correctif
architectural n'est pas fait ici** : il demande d'ajouter un bras au publisher
et de décider d'où viennent les identifiants d'équipes et d'espace. C'est la
carte 352.

## Le verrou

`check-arch` **axe 12**, bloquant : aucun `app_event_bus.send(` hors de
`publier()`. Sans lui, le prochain app event ajouté serait muet et personne ne
le saurait. Les tests sont exemptés — ils simulent un BC amont qui émet, et
n'ont rien à journaliser.

Vérifié sur un cas volontairement fautif, comme l'axe 11 dont la première
écriture ne détectait rien.

## Ce que ça permet

```
grep event_id=01M0…
```

rend l'histoire complète d'un fait : l'émission par son BC d'origine, puis
chaque BC qui y a réagi, dans l'ordre. Avant, on n'avait que la seconde moitié.

## Checklist

- [x] Les onze sites d'émission passent par `publier()`, qui journalise `event`
      et `event_id`
- [x] `domain_event` porté par un span, dans les sept publishers
- [x] L'identifiant journalisé est celui de l'enveloppe **produite** — garanti
      par construction, la fonction ne voit que celle-là, et vérifié par un test
- [x] Test : l'enveloppe part bien sur le bus, la journalisation ne remplace pas
      la publication
- [x] Axe 12 `check-arch` bloquant, vérifié sur un cas volontairement fautif
- [x] `make lint`, `make test` et `make check-arch` passent
- [x] Vérifié en conditions réelles sur une création d'équipe : un seul
      `grep event_id=…` montre l'émission par `team_creation` puis les
      réactions de `teams` et `players`
