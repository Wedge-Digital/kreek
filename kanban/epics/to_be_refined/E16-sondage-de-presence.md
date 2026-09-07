# E16 — Sondage de présence

**État :** `to_be_refined` — 22 cartes prêtes (507 à 528), zéro faite. Elles
couvrent **deux des trois unités** : l'onglet de l'organisateur et la réponse du
coach par e-mail. La troisième — l'encart du coach connecté — n'a pas encore de
cartes. C'est ce qui tient l'épic en `to_be_refined/` : son périmètre n'est pas
entièrement conçu, et non pas une carte qui resterait floue.
Spécifiée par le workflow feature les 2026-09-06 et 07.
**Conception :** `docs/specs/sondage-presence/`

## La fonction

Demander aux coachs, par e-mail, s'ils seront là pour une journée de ligue ;
suivre les réponses ; puis apparier les présents au sort, en évitant les
rencontres déjà jouées.

Elle répond à un défaut d'organisation, pas de calcul : dans une ligue amateur,
on ne sait pas qui vient avant le jour même, et un calendrier tiré à l'avance
produit des matchs que personne ne joue.

Le coach répond **d'un clic depuis sa boîte mail**, sans connexion — un lien par
réponse. L'organisateur voit trois colonnes (présents, absents, sans réponse),
peut saisir à la place de qui l'a appelé, et tire au sort les présents.

## État

**Zéro carte faite.** Les seize cartes sont prêtes, issues des phases 2 à 8 du
workflow sur l'unité `onglet-presences`.

Trois d'entre elles — 507, 508 et 509 — **corrigent l'existant** et ne
concernent pas le sondage : l'algorithme d'appariement du Calendrier n'est ni
aléatoire ni optimal, et il laisse des équipes sans match dans 54 à 58 % des
tirages en milieu de saison. Le sondage s'appuie dessus, donc il le répare
d'abord — et le Calendrier en profite.

Ce qui existait déjà et sera réemployé sans une ligne de plus : les deux domain
events d'appariement et leur publisher, les trois ports vers `teams`, `spaces`
et `match_report`, l'infrastructure d'e-mails avec sa table
`notification_deliveries`, et le patron de jeton opaque de `reset_password`.

## Les cartes

| N° | Carte | Vague |
|---|---|---|
| 507 | Le tirage rejoue plutôt que de laisser une équipe sur le banc | 1 — corriger l'existant |
| 508 | Le Calendrier passe au nouveau tirage | 1 |
| 509 | Écrire un tirage est atomique | 1 |
| 510 | La table des campagnes et son dépôt | 2 — le socle |
| 511 | `PresenceSurvey` I — ouvrir une campagne | 2 |
| 512 | `PresenceSurvey` II — enregistrer une réponse | 2 |
| 513 | `PresenceSurvey` III — valider un tirage | 2 |
| 514 | Les destinataires, et ceux sans adresse | 2 |
| 515 | Le cycle de la campagne — ouvrir, relancer, clore, rouvrir | 3 — les use cases |
| 516 | Poser une réponse | 3 |
| 517 | Tirer au sort, et l'écrire au calendrier | 3 |
| 518 | Défaire, et réparer | 3 |
| 519 | L'onglet Présences et sa barre latérale | 4 — l'écran |
| 520 | Le panneau, six états | 4 |
| 521 | Les neuf actions | 4 |
| 522 | Les tests e2e de l'onglet | 4 |
| 523 | Le dépôt sait retrouver une campagne par son jeton | 5 — le chemin public |
| 524 | La page de réponse sait ce qu'elle a à dire | 5 |
| 525 | La route publique, et le verrou qui la tient | 5 |
| 526 | L'e-mail de sondage et sa relance | 6 — l'expédition |
| 527 | L'expédition remplace le journal qui n'envoyait rien | 6 |
| 528 | Les tests e2e du parcours depuis le lien | 6 |

## Ce qui commande l'ordre

**507 avant 508** — la correction de l'algorithme précède son adoption, et son
test rouge précède la correction. Le cas qui casse — quatre équipes dont trois
paires déjà jouées — n'est couvert par aucun des six tests actuels.

**507 et 509 avant 517** — le tirage du sondage appelle la fonction corrigée et
écrit par la méthode transactionnelle.

**511 avant 512 et 513** — les deux posent des méthodes sur l'agrégat que la
première construit.

**510 avant tout use case** — ils chargent et persistent.

**519 avant 520 avant 521** — la coquille, puis ce qu'elle contient, puis ce qui
la mute.

**523 avant 524 avant 525** — charger, puis nommer, puis afficher.

**526 avant 527** — le mailer rend des gabarits ; sans eux il ne compile pas. Et
**515 avant 527**, la seule dépendance qui remonte d'une unité à l'autre : c'est
elle qui pose le trait d'expédition et son implémentation provisoire, que la 527
remplace.

Le reste est du confort : 514, 516 et 526 se livrent quand leurs dépendances sont
là — la 526 n'en ayant aucune, elle peut partir à tout moment.

## Ce que l'épic ne couvre pas

**Le classement.** Une équipe non appariée — absente, silencieuse ou exemptée —
ne vaut rien : pas de match, pas de points, aucun forfait. Le BC `ranking` n'est
pas touché (R14).

**Le middleware CSRF.** L'investigation a trouvé qu'il n'existe pas, malgré trois
commentaires qui le décrivent. Cela ne bloque rien ici — la route publique de
réponse n'a aucune session à protéger — et mérite sa carte, hors de cette épic.

**Le vert du design system.** `--green` porte 3,3:1 sous du blanc, sous le seuil.
Les maquettes l'assombrissent localement ; corriger le token dépasse ce chantier.

## Terminé quand

Un organisateur ouvre une campagne sur une journée, reçoit des réponses, tire au
sort et retrouve ses rencontres au Calendrier — **sans avoir saisi une seule
présence à la main**.

Le critère porte sur le chemin complet et non sur l'onglet seul : c'est l'e-mail
qui fait l'intérêt de la fonction, et un onglet où l'organisateur coche
quatorze cases lui-même ne résout rien qu'il ne sache déjà faire.
