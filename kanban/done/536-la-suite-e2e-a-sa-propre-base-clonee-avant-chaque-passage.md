# La suite e2e a sa propre base, clonée avant chaque passage

**Priorité : haute — préalable de mesure à la carte 535**
**Dépend de :** rien · **Sans épic**
**Trouvée par :** le raffinage de la carte 535, le 2026-09-07
**Fichiers :** `Makefile`, `scripts/e2e_db.sh` (nouveau), `.env.e2e` (nouveau,
ignoré par git), `tests/e2e/README.md`

## Le constat, mesuré

La suite e2e tourne dans **la base où l'on travaille à la main**, et rien ne la
purge jamais. Après une journée d'exécutions :

| | |
|---|---|
| Poids de la base | **155 Mo**, dont ~94 % de résidus de test |
| `players_proj` | 44 135 lignes |
| `players_events` | 77 222 lignes |
| `event_log` | 46 795 lignes |
| Durée de la suite | 6 min 23 · 6 min 15 après séparation |

### La lenteur n'était pas le problème — correction d'une mesure fausse

Le raffinage avait annoncé **25 % de lenteur**. C'était faux, et l'erreur mérite
d'être écrite : les passages comparés n'exécutaient pas le même nombre de tests.
Les 5 min 15 « sur base neuve » portaient sur **306** tests, les 6 min 40 sur
**368**. La différence venait de la sélection, pas de la base.

À sélection identique — la suite complète, 368 tests :

| Base | Durée |
|---|---|
| accumulée, partagée (155 Mo) | 6 min 23 |
| dédiée, clonée (9 Mo) | 6 min 15 |

**Deux pour cent.** L'accumulation ne ralentissait quasiment pas.

Ce qui justifie la carte est donc ailleurs, et tient toujours :

- **la suite écrivait dans la base de travail** et n'en effaçait jamais rien —
  94 % de résidus après une journée, sans qu'aucun signal ne le dise ;
- **chaque passage part désormais du même état**, ce qui est la condition pour
  que l'enregistrement des échecs de la carte 535 veuille dire quelque chose ;
- la croissance sans borne disparaît.

## Ce que cette carte ne règle pas

**L'instabilité de la suite — carte 535.** Mesuré : sur une base *fraîchement
réinitialisée*, le premier passage a échoué quand même. La suite fabrique
elle-même ses dizaines de milliers de lignes **pendant** qu'elle tourne, et les
plafonds d'attente cèdent sur cette charge-là, pas sur celle héritée de la
veille.

Ce que cette carte apporte à la 535 est autre chose, et c'est ce qui la rend
prioritaire : **des passages comparables**. Aujourd'hui, deux exécutions ne
partent pas du même état, donc on mélange la dérive de la base et la vraie
variabilité. L'enregistrement des échecs que la 535 demande n'est pas
interprétable tant que ce n'est pas réglé.

## Le montage

```
kreek_e2e_gabarit    ← construit une fois : migrations + seed
kreek_e2e            ← recréé depuis le gabarit avant chaque passage
kreek_db             ← la base de travail, que la suite ne touche plus jamais
```

### On clone, on ne crée pas

| Méthode | Durée mesurée | Comment elle vieillit |
|---|---|---|
| `sqlx database reset` + `seed_e2e` | **5 s** | rejoue les migrations — quarante aujourd'hui, le double dans un an |
| `CREATE DATABASE … TEMPLATE` | **< 1 s** | copie de fichiers, insensible à l'histoire du schéma |

Cinq secondes ne sont pas un obstacle en soi — c'est 1,6 % d'un passage. Mais
**ce coût grandit et l'autre non**, et une réinitialisation qu'on paie à chaque
lancement finit par se faire sauter.

### Le gabarit se refait quand il est périmé, pas à chaque fois

On compare le nombre de migrations appliquées dans le gabarit à celui du dossier
`migrations/`. S'il a bougé, on reconstruit ; sinon on clone. **Un gabarit
périmé ferait tourner la suite sur un schéma d'hier sans rien dire**, ce qui
serait pire que la lenteur qu'on corrige.

### Deux détails validés à la mesure

**Couper les connexions avant de remplacer la base.** `DROP DATABASE` échoue sur
une base utilisée. Un `pg_terminate_backend` la libère — vérifié sur
`kreek_db` : le serveur a survécu, son pool s'est reconnecté seul, et le seed
était là.

**Le gabarit ne doit avoir aucune connexion ouverte** au moment du clonage.
Personne ne s'y connecte jamais, mais le script doit le refuser explicitement
plutôt que d'échouer avec un message de PostgreSQL.

## Ce qui a été écarté, et pourquoi

**Purger automatiquement la base de développement avant chaque `make e2e`.**
C'est la réponse évidente, et elle **détruit silencieusement le travail manuel**
du développeur à chaque lancement de la suite. Une commande qui efface sans le
dire est exactement ce que le projet proscrit ailleurs.

**Une purge ciblée** — n'effacer que ce que les tests ont créé. Impraticable :
les fixtures créent des espaces, des compétitions, des équipes et des joueurs
entremêlés aux données manuelles, sans marqueur qui les distingue.

**Se contenter d'un avertissement** — `make e2e` annonce le poids de la base et
suggère une purge. Rend la dette visible sans la traiter, et repose sur la
discipline de chacun.

## Conception

### `scripts/e2e_db.sh`

Une seule responsabilité : rendre `kreek_e2e` identique au gabarit.

1. refuse une URL distante — la garde `refuser_si_distant` du `Makefile` ;
2. si le gabarit manque ou que le compte de migrations a bougé : le construire
   (créer, migrer, seeder) ;
3. couper les connexions à `kreek_e2e`, la supprimer, la recréer depuis le
   gabarit.

### `Makefile`

| Cible | Effet |
|---|---|
| `dev-e2e` | comme `dev-demo`, mais sur `kreek_e2e` |
| `e2e` | appelle `scripts/e2e_db.sh` **avant** pytest |
| `test-impacted` | idem |
| `e2e_gabarit` | force la reconstruction du gabarit |

`.env.e2e` porte l'URL de `kreek_e2e`, sur le patron des autres profils, et le
`.gitignore` l'ignore déjà — il refuse tout `.env*` sauf l'exemple.

### `tests/e2e/README.md`

La section « Prérequis » change de serveur : `make dev-e2e` au lieu de
`make dev-demo`, et `make seed_e2e` disparaît — le clonage s'en charge.

## Le point d'attention

**Le serveur doit tourner sur `kreek_e2e` avant que la suite ne démarre.** Si le
développeur a lancé `make dev-demo`, la suite réinitialise une base que le
serveur ne lit pas, et les tests échouent sur des données absentes — avec un
message qui n'expliquera rien.

`conftest.py` vérifie déjà que le serveur répond ; il doit en plus vérifier
**qu'il répond sur la bonne base**, et le dire clairement sinon. Sans cette
garde, cette carte remplace une lenteur invisible par une confusion invisible.

## Checklist

- [ ] `.env.e2e`, sur le patron de `.env.dev`
- [ ] `scripts/e2e_db.sh` : garde distante, gabarit conditionnel, clonage
- [ ] Détection du gabarit périmé par le compte de migrations
- [ ] `pg_terminate_backend` avant le `DROP`, et refus si le gabarit est occupé
- [ ] Cibles `dev-e2e`, `e2e_gabarit` ; `e2e` et `test-impacted` clonent d'abord
- [ ] `conftest.py` refuse un serveur qui ne tourne pas sur `kreek_e2e`
- [ ] `tests/e2e/README.md` mis à jour
- [x] Deux passages de `make test-impacted` : 6 min 15 et 6 min 22, avec
      respectivement 4 erreurs et 1 échec — tous des cas de la carte 535,
      reproduits nulle part en isolation
- [ ] Corriger les tests qui lancent le binaire : `cargo run` hérite de
      l'environnement, charge `.env.dev` et travaille sur la base de travail
      pendant que les assertions lisent `kreek_e2e` (`test_notification_cron`)
- [ ] Supprimer `kreek_db_gabarit`, laissée sur la machine par la mesure
