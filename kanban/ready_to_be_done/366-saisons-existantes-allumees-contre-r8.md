# R8 n'a jamais été tranchée, et 318 saisons sont notifiantes

**Priorité : haute** — bloquant avant tout déploiement de l'épic E02
**Épic :** E02 — Notifications e-mail de compétition
**Trouvée par :** la revue de déploiement de la branche `demo`
**Fichiers :** une migration nouvelle, `kanban/epics/en_cours/E02-*.md`

## Le problème

La spec est sans ambiguïté (`docs/specs/notifications/README.md:254`) :

> **R8 — Les saisons existantes démarrent éteintes, les nouvelles allumées.**
> Aucune compétition déjà créée ne se met à envoyer des e-mails sans que son
> organisateur l'ait demandé.

La migration `20260822000001_competition_seasons_notifications.sql` ne remplit
pas les lignes existantes. Elle le dit elle-même, en en-tête :

> Les saisons d'avant cette migration démarrent donc actives, et non éteintes
> comme R8 le prévoyait pour elles. […] Ce sera **à trancher avant la 340** :
> assumer, ou corriger la donnée par une migration de rattrapage.

**La 340 est faite. La décision, non.** L'épic porte la même note, sous un titre
« À trancher avant la 340 » que plus rien ne rattrape.

## Le compte, et pourquoi il compte

Sur la base de dev :

```sql
SELECT count(*) FILTER (WHERE notifications IS NULL), count(*)
FROM competition_seasons;
--  318 | 471
```

Trois documents donnent trois chiffres — l'épic annonce 213, la spec ~399, la
base en compte 318. **Personne n'a le bon**, ce qui suffit à dire que la question
n'a pas été rouverte depuis qu'elle a été posée.

`NULL` est lu comme « absent », donc rendu par le défaut serde, qui vaut
**allumé** (`competition_notifications.rs:69`). Les 318 partent avec les quatre
notifications actives.

## Ce que la spec avait prévu, mot pour mot

`configuration/06-domaine.md:131` avertissait déjà :

> Si ce remplissage disparaissait, ce test continuerait de passer pendant que
> ~399 saisons se mettraient à envoyer.

C'est arrivé. La suite est verte, et 318 saisons sont notifiantes. Le test qui
garde R8 vérifie que `{}` se désérialise en quatre `true` — c'est le défaut
« saison neuve », et il est correct. Ce qu'aucun test ne vérifie, c'est que les
lignes **existantes** ne sont pas dans ce cas.

## Conception

### Éteindre, plutôt qu'assumer

Une migration qui pose les quatre à `false` là où `notifications IS NULL`.

```sql
UPDATE competition_seasons
SET notifications = '{"registration_open":false,"round_eve":false,
                      "round_closing":false,"registration_deadline":false}'::jsonb
WHERE notifications IS NULL;
```

**Pourquoi éteindre et non assumer.** L'argument d'assumer serait qu'un
organisateur ayant coché « Activées » dans l'ancien écran attend ses e-mails.
Mais cette parole n'a jamais été tenue — les deux interrupteurs
(`use_mail_notification`, `notify_by_email`) ne branchaient rien, et personne
n'a jamais reçu un seul de ces messages. Les honorer rétroactivement ferait
partir des notifications que plus personne n'attend, sur des saisons dont
certaines sont terminées.

C'est le raisonnement de la spec, et rien depuis ne l'a démenti.

### Un test qui tienne la règle plutôt que la mémoire

Le test de sérialisation ne peut pas voir la donnée. Ce qui peut la voir est une
requête : après migration, `notifications IS NULL` doit rendre **zéro ligne**, et
la colonne devient `NOT NULL` pour que le cas ne puisse plus réapparaître.

Poser la contrainte est ce qui fait la différence entre une correction et un
verrou : sans elle, un `INSERT` qui oublie la colonne recrée le trou en silence.

**À vérifier avant :** que le chemin de création de saison écrit bien la colonne.
Si ce n'est pas le cas, `NOT NULL` casserait la création — poser alors un
`DEFAULT` avec les quatre à `true`, qui est justement le défaut « saison neuve ».

## Checklist

- [ ] Migration de rattrapage — les quatre à `false` où `NULL`
- [ ] `NOT NULL` sur la colonne, avec le `DEFAULT` « neuve » si la création ne
      l'écrit pas
- [ ] Un test d'intégration : après migration, aucune ligne à `NULL`
- [ ] L'épic E02 corrige sa section « À trancher avant la 340 » — la décision est
      prise, elle cesse d'être une question
- [ ] `make check-arch`, `make test`

## Ce que la carte ne couvre pas

**Une reprise des deux anciens interrupteurs comme valeur de départ.** Écartée
ci-dessus, et par la spec avant elle. Si quelqu'un veut rouvrir ce débat, c'est
une autre carte — celle-ci pose le défaut sûr.
