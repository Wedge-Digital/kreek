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

## Ce qui a été décidé en la livrant

**Le regroupement est passé dans le type, pas dans l'implémentation.**
`EnvoiPresence` porte désormais `coach_id`, l'adresse, le nom, et **la liste des
équipes de ce coach**. `expedier` ne peut donc plus envoyer deux messages au même
coach sans boucler franchement de travers, là où un `&[EnvoiPresence]` par équipe
faisait dépendre R1 de la discipline de celui qui écrit l'expéditeur. C'est le
même raisonnement que `RapportEnvoi` sans `Result` : un verrou qui se laisse
oublier n'est pas un verrou.

**`envois_pour` est générique, et c'est la seule copie de la boucle.** La relance
avait sa propre fonction, `envois_des_silencieux`, qui refaisait le même travail
sur `sans_reponse()`. Deux copies auraient donné deux occasions de perdre R1 — et
la relance est celle qu'on regarde le moins. Elle appelle maintenant la même
fonction, avec un autre itérateur.

**`coach_name` s'ajoute à `EquipeSollicitee`, à côté de `coach_label`.** Le
libellé existant vaut « Lepandawan · 2 équipes » : « Salut Lepandawan · 2
équipes, » n'est pas une salutation, et le découper pour retrouver le nom aurait
été une occasion de le perdre. Deux champs, pas un découpage.

**`CampagneAAnnoncer` porte aussi `season_id` et `round_id`**, qui ne s'affichent
nulle part : ils composent la clé du journal. Les faire recharger par l'expédition
lui aurait donné un accès aux dépôts qu'elle n'a pas à avoir.

**L'URL vient du handler**, via `EtiquettesCampagne`, parce que `AppRoutes` vit
dans la couche web. Le contre-exemple existant — `send_due_notifications_use_case`
qui écrit `/app/{}/competitions/{}/{}` à la main — n'a pas été reproduit ; il fait
l'objet de la carte 545.

**Le câblage est dans `context.rs`, pas dans `main.rs`** comme cette carte
l'annonçait : `pool`, `email_service` et `app_url` y sont déjà tous les trois.

## Deux pièges rencontrés, à ne pas réapprendre

**`cargo fmt` déplace le marqueur de l'axe 12.** L'appel d'expédition réparti sur
quatre lignes met le `// arch:ok` hors de portée du contrôle, qui ne lit que la
ligne de l'appel et celle qui la précède. L'adresse passe donc par une variable
pour que l'appel tienne sur une ligne — et le commentaire l'explique sur place.

**L'axe 12 ne dépouille pas les commentaires.** Écrire le nom de la méthode dans
l'explication ci-dessus suffisait à faire échouer la vérification *sur cette
explication*. Déjà rencontré en carte 517, sur `appariement_ecrit.rs`.

## Ce que la 527 laisse ouvert

Les trois compteurs qu'elle produit **n'atteignent aucun écran** :
`presences_actions.rs` fait `Ok(_) => succes()` et les jette. Un organisateur dont
le serveur de messagerie est en panne voit son panneau se recharger normalement et
croit ses e-mails partis. C'est la carte **544**.

## Checklist

- [x] `CampagneAAnnoncer` gagne le nom de la compétition, son URL, les deux dates
      de journée, la saison et la journée ; `EnvoiPresence` gagne le `coach_id`
- [x] Le regroupement par coach **dans le type**, fait par `envois_pour`, partagé
      avec la relance
- [x] `survey_mailer.rs` : `claim`, envoi, `confirm`, un destinataire à la fois
- [x] La construction des URL — `app_url` + `AppRoutes` + jeton + verbe
- [x] Les deux `target_date`, selon `EnvoiKind`
- [x] `context.rs` injecte cette implémentation ; **l'implémentation provisoire de
      la 515 est supprimée** (ses deux seuls appelants étaient là)
- [x] Huit tests `#[sqlx::test]` avec `IEmailService` espionné : un coach à deux
      équipes reçoit **un** e-mail à quatre liens · un second appel rend
      `deja_envoyes` sans rien envoyer · un envoi en échec laisse `sent_at` à
      `NULL` et n'arrête pas la boucle · la relance du jour de l'ouverture part
      quand même · deux fois mardi n'en fait qu'un, mardi puis jeudi en fait deux
      · chaque envoi ne porte qu'une adresse · l'e-mail apostrophe le coach par
      son nom
- [x] `make lint`, `make check-arch`, `make test`
