# Notifications de compétition

Prévenir les coachs par email de ce qui se passe dans une compétition :
ouverture des inscriptions, veille de journée, fin de fenêtre imminente, date
limite d'inscription approchante.

## Décisions prises en phase 1

| Question | Décision |
|---|---|
| Réglage | **Un réglage par notification** — quatre cases indépendantes. Le modèle actuel (`notify_by_email`, un booléen) doit donc changer. |
| Destinataires des emails de journée | **Tous les coachs inscrits**, qu'ils jouent ou non. Ceux qui ont un match y trouvent leur adversaire. |
| Ordonnancement | **Une tâche cron quotidienne**, à heure fixe, dans le fuseau du serveur. |
| Langue | **Français seul.** `emails/en_EN/` a été supprimé — jamais référencé, et sa structure avait divergé sans que personne le voie. |

## Ce que l'investigation a trouvé, et qui change le périmètre

**`notify_by_email` existe déjà** dans `CompetitionInvitations`, avec sa case en
étape 4 du magicien : « Notifier les coachs par email quand la compétition est
ouverte ». Elle est **stockée et jamais lue**. Le choix du créateur est donc
déjà modélisé — il ne fait rien.

**`registration_deadline` existe déjà**, de même que les deux types de journée
(`FixedDate` / `TimeFrame`) avec `date_start` et `date_end` : ils correspondent
exactement aux notifications 2 et 3.

**Il n'existe aucun ordonnanceur.** Ni cron, ni tâche périodique. Trois des
quatre notifications sont temporelles : c'est la brique entièrement neuve, et
celle qui porte les questions difficiles — idempotence, rattrapage, fuseau.

**`CompetitionsAppEvent` ne porte pas la création de saison** — seulement
`CompetitionCreated`, `PairingCreated`, `PairingDeleted`.

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

## Découpage proposé pour les phases 2 à 8 — **à valider**

La fonctionnalité ne se découpe pas en pages, comme le workflow le suppose :
il n'y a qu'un écran, et le reste est un mécanisme d'envoi. Découpage proposé :

| Unité | Contenu |
|---|---|
| `configuration/` | l'écran de réglage dans l'étape 4 du magicien |
| `envoi/` | le service de notification, le cron, les quatre gabarits |

## Progression

| Unité | Front | Back | DTOs | Use cases | Domaine | Intégration | Cartes |
|---|---|---|---|---|---|---|---|
| configuration | | | | | | | |
| envoi | | | | | | | |

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

## Ce que ces règles impliquent pour les phases suivantes

- **R3 crée une table et un agrégat** — c'est la décision la plus structurante,
  et elle tombe en phase 7 (persistance) autant qu'en phase 6 (domaine).
- **R1 impose une journalisation**, donc une observabilité : une notification
  perdue en silence serait indétectable.
- **R2 ne coûte rien** tant que la clé d'idempotence porte la date. Si elle ne la
  portait pas, il faudrait une règle entière — c'est un choix de clé qui décide
  d'un comportement.
