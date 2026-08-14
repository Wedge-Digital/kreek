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

## Règles métier — identifiées en phase 1, **non tranchées**

1. **Le cron ne tourne pas un jour.** Une veille de journée manquée se
   rattrape-t-elle le lendemain — donc le jour même du début — ou est-elle
   perdue ? Envoyer « la journée démarre demain » le jour du début serait pire
   que se taire.
2. **Une journée décalée après l'envoi.** L'organisateur repousse une date après
   que l'email est parti : on renvoie, on annonce le décalage, on ne fait rien ?
3. **Idempotence.** Deux exécutions du cron le même jour ne doivent pas produire
   deux emails. Il faut une trace de ce qui a été envoyé, à quel coach, pour
   quelle journée.
4. **Un coach inscrit sans match.** La maquette prévoit une variante de corps —
   à confirmer comme règle.
5. **Notifications inapplicables.** Pas de fenêtre temporelle, pas de date
   limite : la maquette les grise en disant pourquoi, plutôt que de les masquer.
   Une case absente laisse croire à un oubli ; une case grisée explique.
