# Notifications de compétition

Prévenir les coachs par email de ce qui se passe dans une compétition :
ouverture des inscriptions, veille de journée, fin de fenêtre imminente, date
limite d'inscription approchante.

## Décisions prises en phase 1

| Question | Décision |
|---|---|
| Réglage | **Un réglage par notification** — quatre cases indépendantes. Le modèle actuel (`notify_by_email`, un booléen) doit donc changer. |
| Destinataires des emails de journée | **Tous les coachs inscrits**, qu'ils jouent ou non. Ceux qui ont un match y trouvent leur adversaire. |
| Ordonnancement | **Une tâche cron quotidienne**, à heure fixe, dans le fuseau du serveur, déclenchant une **sous-commande de la CLI kreek** (cf. ci-dessous). |
| Langue | **Français seul.** `emails/en_EN/` a été supprimé — jamais référencé, et sa structure avait divergé sans que personne le voie. |

## Ce que l'investigation a trouvé, et qui change le périmètre

**Deux interrupteurs email morts, pas un** (le second trouvé en phase 2).
`notify_by_email` est en étape 4 — « Notifier les coachs par email quand la
compétition est ouverte » —, et `use_mail_notification` en étape 3, sous la note
« Des rappels seront envoyés aux équipes à l'ouverture et à la fermeture de la
phase » : mot pour mot les notifications 2 et 3. Les deux sont **stockés et
jamais lus**. Le magicien promettait donc déjà cette fonctionnalité à deux
endroits sans la rendre nulle part ; les quatre réglages les absorbent et les
remplacent.

**`registration_deadline` existe déjà**, de même que les deux types de journée
(`FixedDate` / `TimeFrame`) avec `date_start` et `date_end` : ils correspondent
exactement aux notifications 2 et 3.

**Il n'existe aucun ordonnanceur.** Ni cron, ni tâche périodique. Trois des
quatre notifications sont temporelles : c'est la brique entièrement neuve, et
celle qui porte les questions difficiles — idempotence, rattrapage, fuseau.

**`CompetitionsAppEvent` ne porte pas la création de saison** — seulement
`CompetitionCreated`, `PairingCreated`, `PairingDeleted`.

## L'ordonnanceur — une sous-commande de la CLI embarquée

`main.rs` porte déjà un `clap` avec `Serve`, `SeedAccounts` et `SeedE2e` ; une
quatrième sous-commande s'y ajoute, appelée par le cron système.

**Ce qui rend ce choix presque gratuit :**

- `main()` charge la configuration, ouvre le pool **et joue les migrations**
  avant de dispatcher : la commande ne peut pas tourner sur un schéma périmé.
- `compose(cfg, pool) -> AppState` existe — extrait de `run_server` pour le
  harnais de test de la carte 311 — et donne à la CLI tout le câblage
  (repositories, `IEmailService`, ports) sans dupliquer une ligne de `main.rs`.
- Les deux seeds sortent en `exit(1)` sur échec, ce qui rend **R1 observable** :
  une journée manquée devient un code de retour non nul dans les logs du cron,
  au lieu d'un silence.

Même binaire, même configuration, mêmes migrations que le serveur web : aucune
dérive possible entre ce qui sert les pages et ce qui envoie les emails.

**Articulation avec R3** : si le déploiement compte plusieurs instances et que le
cron part sur chacune, c'est la contrainte d'unicité par destinataire qui empêche
les doublons — pas l'ordonnanceur. C'est la troisième fois que cette contrainte
paie, après le décalage de date (R2) et la reprise après panne.

## Le composant d'envoi existe déjà

`src/common/services/email/` expose `IEmailService` — `send(to, subject, html)` —
avec `ResendMailService` (API Resend) et `ConsoleEmailService` pour le dev,
choisis par configuration. C'est un **service partagé, pas un BC** ; `auth` s'en
sert pour le mot de passe oublié. `competitions` n'a qu'à se le faire injecter
dans son contexte.

**Piège dans sa signature** : `to: Vec<String>` invite à expédier une
notification de journée en un seul appel pour trente coachs. Ce serait mettre les
trente adresses en clair dans l'en-tête que chacun reçoit — un annuaire de
l'espace distribué à tout le monde. R4 impose de toute façon un corps
personnalisé : **un envoi par destinataire**, jamais groupé.

## Maquettes — phase 1 validée

| Maquette | Notification |
|---|---|
| `assets/rawpages/email/invitation-competition.html` | ouverture des inscriptions |
| `assets/rawpages/email/email-journee-demain.html` | veille de journée |
| `assets/rawpages/email/email-fin-de-journee.html` | avant-veille de clôture |
| `assets/rawpages/email/email-date-limite-inscription.html` | J-3 avant la date limite |
| `assets/rawpages/html/competition-notifications-config.html` | l'écran de réglage |

Le standard visuel : dégradé `#003049 → #555770` — celui de la page
compétition, pas celui du détail d'article —, logo `email-logo.png` en 200×81,
polices Roboto et Roboto Slab, bouton bleu plein. L'orange reste un accent,
jamais une surface d'action. Toutes les couleurs sont des tokens de
`common.css`.

`invitation-competition.html` préexistait et a été harmonisée : laisser deux
styles aurait fait un univers à deux vitesses.

## Découpage des phases 2 à 8 — validé

La fonctionnalité ne se découpe pas en pages, comme le workflow le suppose : il
n'y a qu'un écran, et le reste est un mécanisme d'envoi.

| Unité | Contenu | Phase 2 |
|---|---|---|
| `configuration/` | l'écran de réglage dans l'étape 4 du magicien | oui |
| `envoi/` | service de notification, cron, journal d'envois, quatre gabarits | **sans objet** |

**`envoi/` n'a pas de phase 2, et ce n'est pas un oubli.** Un email n'a pas
d'architecture front : ni composition HTMX, ni événements DOM, ni swap. C'est
inscrit dans le tableau plutôt que laissé vide, pour la raison même qui a fait
trancher R5 — une case vide laisse croire à un trou, une case renseignée
explique.

**`configuration/` passe en premier, parce que `envoi/` lit ce qu'elle écrit.**
Le modèle actuel est un booléen unique (`notify_by_email`, stocké et jamais lu)
qui devient quatre réglages indépendants. Tant qu'il n'existe pas, le service
d'envoi n'a rien à interroger pour savoir s'il doit envoyer ; l'ordre inverse
obligerait à inventer une forme provisoire puis à la refaire.

**Le journal d'envois de R3 appartient à `envoi/`**, en phase 7 : c'est une
persistance du mécanisme, pas un réglage, et la contrainte d'unicité qui le
garantit se conçoit avec les tables, pas avec l'écran.

Découpage écarté : **une unité par notification.** Les quatre partagent le
mécanisme entier — cron, journal, résolution des destinataires, expédition — et
ne diffèrent que par leur déclencheur temporel et leur gabarit. Les séparer
aurait fait quatre fois les phases 3 à 7 pour un seul mécanisme.

## Progression

| Unité | Front | Back | DTOs | Use cases | Domaine | Intégration | Cartes |
|---|---|---|---|---|---|---|---|
| configuration | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ⏸ |
| envoi | — | ✅ | 🚧 | | | | |

`—` : sans objet, cf. ci-dessus. `⏸` : les cartes des **deux** unités sont
produites ensemble, à la fin de `envoi/`.

**Pourquoi les cartes attendent.** `configuration/` livrée seule n'enverrait
aucun email : personne ne lirait la colonne. Ce serait un **troisième
interrupteur email mort**, mieux dessiné que les deux qu'il remplace et tout
aussi inerte — précisément le défaut que cette fonctionnalité corrige. Les
cartes des deux unités sortiront donc en une fois, ordonnées de sorte que rien
ne soit livrable tant que la chaîne n'est pas complète.

## Règles métier — tranchées en phase 1

### R1 — Une notification manquée est perdue, et journalisée

Si le cron n'a pas tourné, la fenêtre est passée : on n'envoie rien, et
l'incident est tracé. Envoyer « la journée démarre demain » le jour même du
début serait **faux** ; se taire est moins grave que mentir.

Écarté : le rattrapage avec un texte adapté (« démarre aujourd'hui »), qui
doublerait chaque gabarit et obligerait le service à savoir de combien il est en
retard.

### R2 — Un décalage de date réarme la notification, sans règle supplémentaire

La trace d'envoi est clée sur la **date visée**, pas seulement sur la journée.
Une date qui change produit donc une clé nouvelle : la veille de la nouvelle
date, l'email repart de lui-même.

C'est le bénéfice le moins évident de R3 — l'idempotence règle le décalage sans
qu'on écrive quoi que ce soit pour lui. Écarté : un email « la date a changé »,
qui aurait été une cinquième notification, avec son gabarit, son réglage et ses
tests.

### R3 — L'idempotence est garantie **par destinataire**, et par la base

```sql
notification_deliveries
    notification_type
    competition_id / round_id
    target_date
    coach_id
    sent_at
    UNIQUE (notification_type, round_id, target_date, coach_id)
```

**C'est la contrainte d'unicité qui garantit, pas le code applicatif.** Une
relance manuelle, un redémarrage ou un second serveur ne doivent pas produire un
second email — et aucune logique applicative ne tient cette promesse aussi bien
qu'un index unique.

Le grain par destinataire, et non par lot : un coach inscrit tardivement reçoit
encore son email, et une panne au milieu d'un envoi ne laisse pas le lot marqué
« fait » alors que la moitié des coachs n'a rien reçu.

### R4 — Tous les coachs inscrits reçoivent l'email de journée, avec deux corps

Conséquence de la décision de destinataires : celui qui a un match y trouve son
adversaire, les autres une ligne de calendrier. Un seul gabarit, deux corps.

### R5 — Une notification inapplicable est grisée, avec son motif

Pas de fenêtre temporelle, pas de date limite : la case reste visible, désactivée,
et dit pourquoi. **Une case absente laisse croire à un oubli ; une case grisée
explique.**

### R6 — Une notification cochée puis rendue inapplicable reste cochée

Apparue en phase 2 (`configuration/02-front.md`). L'organisateur coche « date
limite d'inscription » puis efface la date : la case reste **cochée et grisée**,
la valeur stockée intacte.

Décocher détruirait un choix explicite en réaction à un geste qui n'a rien à
voir, sans que l'organisateur le voie. Le grisage dit déjà « sans effet
aujourd'hui », et l'intention est conservée pour le jour où une date reviendra.

Même préférence que R1 : ne rien faire silencieusement plutôt qu'agir à côté de
ce qui a été demandé.

### R7 — Le périmètre des destinataires est toujours borné par l'espace

Sans cette borne, « tous ceux qui peuvent s'inscrire » désigne **la plateforme
entière** : chaque compétition créée dans un espace notifierait tous les coachs
de tous les espaces. Ce n'est jamais l'intention.

| Notification | Destinataires |
|---|---|
| Ouverture des inscriptions | mode `invitation` : les coachs invités. Mode `open` : **les membres de l'espace**, jamais au-delà |
| Veille de journée | les coachs inscrits à la compétition |
| Fin de journée imminente | les coachs inscrits à la compétition |
| Date limite d'inscription | ceux qui peuvent encore s'inscrire et ne l'ont pas fait : invités non inscrits, ou membres de l'espace non inscrits |

Les notifications 2 et 3 sont bornées par construction — on ne peut être inscrit
à une compétition sans être dans son espace. **Le risque porte sur 1 et 4 en mode
`open`**, les deux seuls cas où le périmètre se déduit d'une règle d'accès plutôt
que d'une liste nominative.

#### Ce qu'il faut pour la tenir, et où c'est

L'appartenance vit dans `spaces__user_space`, les adresses dans
`spaces__user_cache`, et `list_members_for_space.sql` fait déjà exactement la
jointure voulue — le tout **possédé par `spaces`**. `competitions` y accède donc
par le port `ICompetitionSpaceMemberPort`, qui existe avec son adapter, augmenté
d'une méthode de listing. Jamais par une requête directe.

#### Le piège à connaître avant `envoi/`

Une migration a créé `competitions__user_cache`, `competitions__space_cache` et
`competitions__user_space_cache` — avec les emails et l'appartenance, dans le BC
`competitions`. **Ces trois tables ne sont ni lues ni écrites nulle part.**

C'est exactement ce qu'on trouve en cherchant « où sont les emails », et qu'on
branche sans voir qu'elles sont vides : aucune notification ne partirait, et
aucune erreur ne le dirait.

#### Entorse existante, signalée et non traitée

`src/app/competitions/io/repository/sql/competitions/find_competition_by_id.sql`
fait `LEFT JOIN spaces__user_cache` : un BC qui requête la table d'un autre,
contre la règle de souveraineté des données. Hors sujet ici, mérite sa carte.

### R8 — Les saisons existantes démarrent éteintes, les nouvelles allumées

Apparue en phase 4 (`configuration/04-dtos.md`). Aucune compétition déjà créée ne
se met à envoyer des emails sans que son organisateur l'ait demandé ; toute
saison créée après la livraison arrive avec les quatre notifications actives, que
le magicien montre et qu'on peut décocher.

Écarté : reprendre les deux interrupteurs morts comme valeur de départ. L'argument
était sérieux — l'UI promettait les emails et proposait « Activées » par défaut,
donc les honorer aurait tenu une parole donnée. Mais cette parole n'a jamais été
tenue, et personne n'a jamais vu un seul de ces emails : la réactiver
rétroactivement sur ~399 saisons ferait partir des messages que plus personne
n'attend.

**Conséquence technique, à ne pas perdre :** la migration doit **écrire
explicitement** les quatre `false` sur toutes les lignes existantes. Si l'absence
de valeur servait de défaut, `NULL` signifierait à la fois « ancienne saison,
donc éteint » et « saison neuve, donc allumé », sans rien dans la ligne pour les
distinguer — le `status` n'y suffit pas, `invitations_configured` désignant aussi
bien une saison abandonnée en cours de magicien qu'une saison d'avant la
migration. Une fois les lignes remplies, `NULL` veut dire « créée après », et
rien d'autre.

### R9 — L'activation n'est jamais rétroactive

Les réglages sont modifiables sur une compétition démarrée — c'est l'objet même
du mode auto-save de l'écran d'admin. **Activer une notification ne déclenche
aucun envoi passé.** Cocher « veille de journée » en octobre ne rejoue pas les
six journées de septembre.

**Cela ne va pas de soi, c'est même le contraire.** R3 crée un journal des
envois, et un journal des envois appelle naturellement l'implémentation
« cherche ce qui aurait dû partir et n'est pas parti, puis envoie-le » — qui est
exactement rétroactive.

La règle qui l'empêche : **le cron ne regarde jamais en arrière.** Il calcule les
fenêtres dues *aujourd'hui*, et le journal ne sert **qu'à empêcher les doublons,
jamais à détecter des trous**.

R1 — une journée manquée est perdue — en devient un cas particulier, et non une
décision indépendante : ne pas rattraper, c'est ne pas regarder en arrière.

**Cas à signaler** : l'ouverture des inscriptions n'est pas une fenêtre de date,
contrairement aux trois autres — elle se déclenche sur un fait, la saison
s'ouvre. Quelle que soit la manière dont `envoi/` la câblera, cocher la case
trois semaines plus tard ne doit pas la faire partir : l'ouverture a eu lieu.

### R10 — Le fuseau retenu est celui du serveur, et le sélecteur disparaît

Apparue en phase 3 de `envoi/`. `ScheduleConfig` porte un `schedule_timezone`,
saisi à l'étape 3 du magicien avec son sélecteur et `Europe/Paris` par défaut,
**stocké et lu par personne** — un troisième réglage mort, après les deux
interrupteurs email.

La phase 1 avait décidé que le cron tournerait dans le fuseau du serveur. Les
deux ne pouvaient pas être vrais ensemble : soit on honore le fuseau déclaré,
soit on retire le réglage. **Le sélecteur disparaît**, comme les deux autres.

Ce que cela coûte : les ligues à cheval sur plusieurs fuseaux n'auront pas de
réglage. Ce que cela évite : un réglage que l'interface propose et que rien
n'applique — le défaut même que cette fonctionnalité corrige ailleurs.

**À ne pas laisser derrière** : le VO `Timezone`
(`shared_kernel/bloodbowl/timezone.rs`) n'a aucun autre utilisateur. Il devient
orphelin le jour où le champ disparaît.

## Ce que ces règles impliquent pour les phases suivantes

- **R3 crée une table et un agrégat** — c'est la décision la plus structurante,
  et elle tombe en phase 7 (persistance) autant qu'en phase 6 (domaine).
- **R1 impose une journalisation**, donc une observabilité : une notification
  perdue en silence serait indétectable.
- **R2 ne coûte rien** tant que la clé d'idempotence porte la date. Si elle ne la
  portait pas, il faudrait une règle entière — c'est un choix de clé qui décide
  d'un comportement.
- **R9 interdit une implémentation naturelle du journal de R3.** Le journal
  répond à « celui-ci a-t-il déjà reçu ? », jamais à « qu'est-ce qui manque ? ».
  Les deux questions se posent sur la même table et donnent des systèmes opposés.
- **R7 fait de la résolution des destinataires un travail à part entière**, avec
  son port et ses quatre cas. Ce n'est pas « lire une liste » : deux des quatre
  notifications s'adressent à des gens qui ne sont *pas* inscrits.
