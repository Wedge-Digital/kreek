# L'expédition remplace le journal qui n'envoyait rien

**Priorité : haute — c'est le dernier maillon du parcours**
**Épic :** E16 — Sondage de présence
**Dépend de :** 526 (les gabarits), 515 (le trait et son implémentation provisoire)
**Fichiers :** `src/app/competitions/io/email/survey_mailer.rs`, `src/main.rs`

## L'objectif

Implémenter `ISurveyMailer` pour de bon, et **retirer l'implémentation qui
journalisait sans envoyer**, posée par la carte 515 pour que l'onglet soit
utilisable avant cette unité.

## Le protocole — `claim` puis `confirm`, jamais l'inverse

`claim` réserve le créneau **avant** l'envoi : c'est l'index unique de
`notification_deliveries` qui départage deux appels parallèles, et zéro ligne
rendue signifie « déjà envoyé ». La base tranche, le code n'arbitre rien.

Entre les deux, la ligne existe avec `sent_at` à `NULL`. Si l'envoi échoue, elle
**reste** dans cet état : un échec constaté, que R20 veut journalisé et qu'une
relance rattrape.

**Un envoi par destinataire, jamais groupé.** Le journal est clé par
destinataire ; un envoi groupé rendrait impossible de savoir qui a reçu quoi, et
une relance ne saurait plus qui rattraper.

## La clé du journal, et ce qu'elle empêche

| Envoi | `target_date` | Effet |
|---|---|---|
| initial | **l'échéance de la campagne** | un seul envoi par campagne et par coach |
| relance | **le jour de l'envoi** | une par jour : le double clic ne part pas deux fois, mais on peut relancer mardi puis jeudi |

Prendre l'échéance pour la relance aussi l'aurait **bloquée définitivement dès
la seconde**, sans le dire : l'organisateur aurait cliqué « Relancer », vu
« 0 envoyé », et cherché la panne.

## Qui reçoit quoi

| Envoi | Destinataires |
|---|---|
| initial | tous ceux qui ont une adresse — R3 écarte les autres, sans bloquer le lancement |
| relance | ceux dont la réponse est `SansReponse` **et** qui ont une adresse |

**La relance ne relit pas le journal pour savoir qui rattraper.** Elle part de
`survey.sans_reponse()`, qui répond à la vraie question — « qui n'a pas
répondu » — et non à « qui n'a pas reçu ». Un coach qui a reçu l'e-mail et n'a
pas cliqué est même le cas principal. Le journal empêche le doublon du jour, pas
la relance.

## R20 gouverne le retour

`EnvoiOutcome { envoyes, deja_envoyes, echecs }`, et **aucune variante d'erreur
qui remonterait**. Un serveur de messagerie indisponible ne doit pas empêcher une
campagne de s'ouvrir : l'organisateur peut saisir à la main (R6), et le coach
connecté répondre depuis l'encart.

## Ce que la 526 laisse ouvert — relevé en la livrant

**Trois champs manquent au trait, et c'est ici qu'ils s'ajoutent.**
`PresenceSurveyEmail` demande `competition_name`, `competition_url`,
`date_start` et `date_end` ; `CampagneAAnnoncer` ne porte que `round_name` et
`deadline`. Rien à deviner dans l'implémentation : ce sont des données que le use
case a sous la main au moment où il appelle `expedier`, et les faire charger par
l'expéditeur lui donnerait un accès aux dépôts qu'il n'a pas à avoir.

**`EnvoiPresence` a perdu le `coach_id`, dont `DeliveryKey` a besoin.**
`EquipeSollicitee` le porte (`survey_roster_service.rs:36`), `envois_pour` ne le
recopie pas — la 515 n'en avait pas l'usage. La clé du journal étant
`(type, saison, journée, date, coach_id)`, sans lui il n'y a pas de `claim`
possible. À rajouter au DTO, pas à retrouver par l'adresse.

**Le regroupement par coach est un travail explicite, et son oubli ne ressemble
pas à un doublon.** `expedier` reçoit **un `EnvoiPresence` par équipe** ; R1 veut
un message par coach. Si l'implémentation boucle naïvement sur les envois, le
premier `claim` du coach passe et **le second rend zéro ligne** : sa deuxième
équipe n'a jamais ses boutons, et le compte rendu affiche « déjà envoyé » — un
message rassurant pour une équipe qu'on vient de perdre. Grouper par adresse
avant de rendre le gabarit, et rendre **un** `EquipeLigneVm` par équipe du
groupe.

## Checklist

- [ ] `CampagneAAnnoncer` gagne le nom de la compétition, son URL et les deux
      dates de journée ; `EnvoiPresence` gagne le `coach_id`
- [ ] Le regroupement par adresse **avant** le rendu — un message, N paires
- [ ] `survey_mailer.rs` : `claim`, envoi, `confirm`, un destinataire à la fois
- [ ] La construction des URL — `app_url` + chemin public + jeton + verbe
- [ ] Les deux `target_date`, selon `EnvoiKind`
- [ ] `main.rs` injecte cette implémentation ; **l'implémentation provisoire de
      la 515 est supprimée** (vérifier qu'elle n'a plus d'appelant)
- [ ] Tests avec `IEmailService` simulé : un coach à deux équipes reçoit **un**
      e-mail · un second appel ne renvoie rien · un envoi en échec laisse
      `sent_at` à `NULL` et n'échoue pas l'ouverture
- [ ] `make lint`, `make check-arch`, `make test`
