# Sentry — l'alerte que le journal ne donne pas

**Priorité : à définir**
**Statut : à raffiner**
**Voisin de :** l'épic E11, close — « Savoir ce qui se passe en production »

## Pourquoi, alors que le journal existe

E11 a donné de quoi **enquêter** : journal structuré, `rid` repris dans
`x-request-id`, corrélation d'un événement à toutes ses réactions,
instrumentation de chaque use case, `JournalDePanic` qui rattrape les panics
(`main.rs:627`).

Il manque l'autre moitié : **être prévenu**. Aujourd'hui, une erreur en
production n'existe que si quelqu'un ouvre `docker logs` et cherche. Un coach
qui se prend un 500 le samedi soir ne le signale pas toujours, et rien ne le
remonte.

Sentry ne remplace pas le journal — il le complète : le journal reste la source
de vérité, Sentry est la sonnette.

## Le piège que ce projet connaît déjà

`CLAUDE.md` le dit sous « le réflexe à avoir » : devant une couche tierce, la
question n'est pas « est-ce que ça journalise ? » mais **« sous quelle cible, à
quel niveau, et qu'est-ce qui le vérifie ? »**. Trois fois l'épic s'est fait
prendre — `TraceLayer`, `CatchPanicLayer`, `#[instrument]` — parce qu'une
bibliothèque émet sur son propre nom, hors du filtre `kreek=…`.

Sentry pose exactement la même question à l'envers : **quels événements
lui parviennent réellement ?** Un `sentry-tracing` branché sur un filtre qui
ne laisse passer que `kreek::` ne verra jamais une erreur de `sqlx` ou d'`axum`.
Il faudra le vérifier par un test d'envoi délibéré, pas par lecture du code.

## Ce qu'il faut décider

**Où il tourne.** SaaS ou auto-hébergé. Ce n'est pas qu'une question de coût :
les événements partent chez un tiers, avec ce qu'ils transportent.

**Ce qui part.** `send_default_pii` doit rester à `false`, mais ça ne suffit
pas : les adresses des coachs sont des données réelles — le dépôt a déjà expulsé
des exports de base pour cette raison (commit `2bd45c3`), et `email_masque.rs`
existe pour ne pas les écrire en clair. À vérifier point par point : ce que le
contexte d'un événement Sentry emporte, ce que les `Secret<T>` masquent
réellement une fois sérialisés, et si le corps des requêtes est capturé.

**Ce qu'on capture.** Erreurs seulement, ou aussi les traces de performance
(`traces_sample_rate`) ? La seconde option double le volume et la facture, et
recoupe ce que le journal donne déjà.

**Comment on est prévenu.** Sentry sait envoyer des e-mails ; le projet a déjà
son `IEmailService` et ses gabarits. À trancher : alerte native Sentry, ou
routage par nos propres moyens.

**La release.** Sans `release` renseignée avec le SHA du commit, un événement ne
dit pas quelle version plante. À câbler au moment du build.

## Ce qu'il faudra brancher

| Point | Où |
|---|---|
| `sentry::init` avant tout le reste | `main.rs`, à côté de `init_journal` |
| Couche `tracing` → Sentry | à composer avec le `Registry` existant et `UseCaseJournal` |
| Middleware de requête | dans l'ordre déjà fixé — `TraceLayer → SessionLayer → AuthLayer → Csrf` |
| Panics | `JournalDePanic` existe : il doit aussi pousser vers Sentry |
| Configuration | `SENTRY__DSN`, format `<SECTION>__<CLÉ>` **sans préfixe** — un `APP__` serait ignoré en silence |

**DSN absent : que se passe-t-il ?** En local, Sentry désactivé va de soi. En
production, un DSN manquant ne doit pas donner un démarrage silencieux et une
surveillance inexistante — c'est le défaut qu'a corrigé `make audit`, où une
étape sautée passait pour verte. Le comportement doit être bruyant quand `CI` ou
l'environnement de production est posé.

## Vérifier les versions avant de s'engager

`sentry`, `sentry-tracing` et `sentry-tower` suivent les versions de `tower`,
`tower-http` et `tracing-subscriber`. Le projet est sur axum 0.8 / tower-http
0.6 / tracing-subscriber 0.3 : la compatibilité se vérifie avant de promettre
quoi que ce soit, et `make audit` aura son mot à dire sur les dépendances
transitives ajoutées.

## À trancher avant de passer en `ready_to_be_done`

- [ ] SaaS ou auto-hébergé
- [ ] Erreurs seules, ou traces de performance
- [ ] Le niveau à partir duquel un événement part — `error`, ou `warn` aussi
- [ ] Politique PII : ce qui est masqué, et qui le vérifie
- [ ] Comportement au démarrage sans DSN, par environnement
- [ ] Le test d'envoi délibéré qui prouve qu'une erreur hors `kreek::` arrive bien
