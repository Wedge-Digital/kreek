# Phase 5 — Use cases : la réponse du coach

**Entrée** : `04-dtos.md` validé — un DTO de chemin, trois structs de template,
deux structs d'e-mail, le service d'hydratation.

## Aucun use case nouveau, et c'est le résultat de la phase

La réponse par jeton appelle **`record_answer_use_case`**, écrit en unité 1, avec
`Repondant::Coach`.

R1 porte sur l'équipe et jamais sur le coach : le jeton désigne une réponse, donc
un `TeamId` dans une campagne — exactement ce que la commande attend. Un use case
propre à la route publique aurait fait un second endroit où tenir R13, R19 et
R21, pour un chemin d'entrée qui ne décide rien de plus.

Cette phase spécifie donc deux choses que personne d'autre ne porte : **où
atterrissent les refus du domaine**, et **le contrat de l'expédition**.

## Où atterrissent les refus

Le handler enchaîne : charger par jeton, enregistrer, hydrater, rendre. Chaque
issue tombe sur l'un des trois gabarits — jamais sur une page d'erreur, jamais
sur un `500`.

| Issue de `record_answer` | Page rendue |
|---|---|
| `Ok(_)` | présence confirmée / absence enregistrée |
| jeton introuvable (avant le use case) | lien inconnu (R26) |
| `SurveyClosedForCoach` (R21) | sondage clos, motif « échéance » ou « décision » |
| `RoundFrozenByReport` (R13) | sondage clos, motif « journée déjà jouée » |
| `TeamNotInSurvey` (R19) | **ne peut pas se produire** — cf. ci-dessous |
| erreur de dépôt | `500`, journalisée |

**R19 est inatteignable par ce chemin, et il faut le dire plutôt que le coder.**
Le jeton *est* la désignation de la réponse : `find_by_token` ne rend une
campagne que parce qu'une de ses réponses porte ce jeton. L'équipe est donc dans
la campagne par construction. Le refus reste dans le domaine — il protège le
chemin de l'organisateur, où le `TeamId` vient d'un formulaire — mais le handler
public le traite comme une erreur de dépôt, pas comme un état de page.

Une quatrième page « cette équipe n'est pas dans la campagne » aurait été un
écran que personne ne peut atteindre, et que personne n'aurait donc jamais
vérifié.

## Le contrat de l'expédition

`ISurveyMailer` est déclaré par la couche applicative en unité 1 (carte 515) ;
c'est ici qu'on dit ce que son implémentation doit tenir.

```rust
async fn send_survey_emails(&self, survey: &PresenceSurvey, destinataires: &[Destinataire], kind: EnvoiKind) -> EnvoiOutcome
pub enum EnvoiKind { Initial, Relance }
```

**Un envoi par destinataire, jamais groupé.** C'est R7 de la spec
`notifications`, et elle vaut ici pour la même raison : le journal est clé par
destinataire, donc un envoi groupé rendrait impossible de savoir qui a reçu quoi
— et une relance ne saurait plus qui rattraper.

**`claim` puis `confirm`, jamais l'inverse.** `claim` réserve le créneau *avant*
l'envoi ; l'index unique départage deux appels parallèles, et zéro ligne rendue
signifie « déjà envoyé ». Si l'envoi échoue, la ligne reste avec `sent_at` à
`NULL` : un échec constaté, que R20 veut journalisé et qu'une relance rattrape.

**Les URL sont construites ici**, pas dans le gabarit : `app_url` + le chemin
public + le jeton + le verbe. Un gabarit qui les assemblerait mettrait la forme
du lien dans du HTML d'e-mail, hors de portée de tout test.

**R20 gouverne le retour** : `EnvoiOutcome { envoyes, deja_envoyes, echecs }`, et
aucune variante d'erreur qui remonterait. Un serveur de messagerie indisponible
ne doit pas empêcher une campagne de s'ouvrir — l'organisateur peut saisir à la
main (R6) et le coach connecté répondre depuis l'encart.

## Qui reçoit quoi

| Envoi | Destinataires | D'où ils viennent |
|---|---|---|
| initial | tous ceux qui ont une adresse | `survey_roster_service` (unité 1), R3 écarte les autres |
| relance | ceux dont la réponse est `SansReponse` **et** qui ont une adresse | `survey.sans_reponse()`, croisé aux adresses |

**La relance ne relit pas le journal pour savoir qui rattraper.** Elle part de
l'état des réponses, qui est la vraie question — « qui n'a pas répondu » — et non
de « qui n'a pas reçu ». Un coach qui a reçu l'e-mail et n'a pas cliqué doit être
relancé ; c'est même le cas principal. Le journal, lui, empêche le doublon du
jour, pas la relance.

## La clé du journal — une décision à ne pas laisser au hasard

`DeliveryKey` porte `{ notification_type, season_id, round_id, target_date,
coach_id }`. Les deux types sont fixés en phase 3 ; reste `target_date`.

| Envoi | `target_date` | Ce que ça permet, et ce que ça bloque |
|---|---|---|
| initial | **l'échéance de la campagne** | un seul envoi initial par campagne et par coach — R2 garantissant une campagne par journée, c'est bien « une fois » |
| relance | **le jour de l'envoi** | une relance par jour et par coach : le double clic ne part pas deux fois, mais l'organisateur peut relancer un mardi puis un jeudi |

**Prendre l'échéance pour la relance aussi aurait bloqué la seconde relance**,
définitivement et sans le dire — l'organisateur aurait cliqué « Relancer », vu
« 0 envoyé », et cherché la panne. Prendre le jour de l'envoi pour l'initial
aurait au contraire permis de rouvrir la campagne et de tout réexpédier le
lendemain, ce que R7 rend inutile : rouvrir réarme les anciens liens, il n'y a
rien à réémettre.

C'est la même règle appliquée deux fois : **la clé dit ce qu'on veut empêcher.**

## Ce que cette phase ne fait pas

**Elle ne touche pas au domaine.** L'agrégat a été conçu d'un bloc en phase 6 de
l'unité 1 pour les trois unités ; `reponse_par_jeton`, `enregistrer` et `statut`
l'attendaient déjà. Il n'y a pas de phase 6 propre à cette unité — sa page
`06-domaine.md` sera une redirection, pas un fichier vide.

**Elle n'émet aucun événement.** Personne hors du BC n'a à savoir qu'un coach a
répondu.

## Règle métier apparue en phase 5

### R27 — Une page qui ne prend plus de réponse dit laquelle des trois causes

Apparue en phase 5, en cherchant où tombe `RoundFrozenByReport`.

Trois situations distinctes ferment le chemin du coach, et la phase 4 les faisait
toutes tomber sur le même écran « Le sondage est clos » :

| Cause | Ce que le coach doit comprendre |
|---|---|
| l'échéance est passée (R23) | c'est la date qui a tranché, pas une panne |
| l'organisateur a clos (R23) | la décision est humaine, il peut la discuter |
| la journée est déjà jouée (R13) | il n'y a plus rien à organiser, sa réponse n'a plus d'objet |

**La troisième n'est pas une clôture.** La campagne peut être ouverte, son
échéance à venir, et la journée pourtant figée par un rapport publié — il suffit
qu'un match ait été joué en avance, ce qu'une ligue amateur fait couramment. Dire
« le sondage est clos » serait alors faux, et l'échéance affichée juste en
dessous le démentirait à l'écran.

`PresenceClosedTemplate` gagne donc un `motif`, et la phase 4 est corrigée en
conséquence.

**Cela ne contredit pas R26.** Là-bas, on tait ce qu'on sait d'un jeton dont on
ignore s'il appartient à quelqu'un ; ici le porteur du jeton est légitime, et lui
expliquer pourquoi son clic n'a rien changé est le minimum.
