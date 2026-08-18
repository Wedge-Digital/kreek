# `app` — Le versant émission des app events entre dans le journal

**Priorité : moyenne**
**Dépend de :** carte 345 (les listeners sont instrumentés — celle-ci fait la
symétrie)
**Fichiers :** les sept `src/app/*/io/app_events/app_event_publisher.rs`

## Le problème

Les 19 listeners ouvrent désormais un span portant `event` et `event_id`. Le
versant **émission**, lui, est resté muet — ou plutôt bavard sans contexte :

```
INFO kreek::app::team_creation::io::app_events::app_event_publisher:
     team_creation_app_event_publisher: relaying TeamSubmitted to app bus
```

Cette ligne ne porte **aucun identifiant**. On sait qu'un événement a été
relayé, mais pas lequel — impossible de la relier aux spans des listeners qui
vont réagir.

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

## Ce qu'il faut faire

Dans chacun des sept publishers, remplacer la ligne actuelle par une ligne
portant les trois informations qui rendent la suite lisible :

| Champ | Valeur |
|---|---|
| `event` | le type de l'app event produit — celui que les listeners verront |
| `event_id` | l'identifiant de l'enveloppe **produite** |
| `domain_event` | le type du domain event d'origine, qui porte souvent un autre nom |

Le format de champs suit celui des listeners (`event=`, `event_id=`), pour
qu'un même `grep event_id=…` ramène l'émission **et** toutes les réactions.

Les sept publishers partagent la même structure ; la modification est
mécanique. Ils ne prennent pas de span : ils n'appellent rien qui journalise en
aval, une ligne suffit.

## Ce que ça permettra

```
grep event_id=01M0…
```

rend l'histoire complète d'un fait : l'émission par son BC d'origine, puis
chaque BC qui y a réagi, dans l'ordre. Aujourd'hui on n'a que la seconde
moitié.

## Checklist

- [ ] Les sept publishers journalisent `event`, `event_id` et `domain_event`
- [ ] L'identifiant journalisé est bien celui de l'enveloppe **produite** —
      vérifié en comparant avec le span d'un listener sur le même fait
- [ ] Vérifié en conditions réelles sur une création d'équipe : un seul
      `grep event_id=…` montre l'émission par `team_creation` puis les
      réactions de `teams` et `players`
- [ ] `make test` et `make check-arch` passent
