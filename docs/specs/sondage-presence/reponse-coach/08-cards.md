# Phase 8 — Les cartes : la réponse du coach

**Entrée** : les phases 3 à 7 validées, la 6 sans objet.

**Épic** : E16 — Sondage de présence. L'unité y ajoute six cartes, portant le
total à vingt-deux.

## Les six cartes

| N° | Carte | Vague | Dépend de |
|---|---|---|---|
| 523 | Le dépôt sait retrouver une campagne par son jeton | 5 — le chemin public | 510 |
| 524 | La page de réponse sait ce qu'elle a à dire | 5 | 523 |
| 525 | La route publique, et le verrou qui la tient | 5 | 523, 524, 516 |
| 526 | L'e-mail de sondage et sa relance | 6 — l'expédition | — |
| 527 | L'expédition remplace le journal qui n'envoyait rien | 6 | 526, 515 |
| 528 | Les tests e2e du parcours depuis le lien | 6 | 525, 527 |

## Ce qui commande l'ordre

**523 avant 524 avant 525** — charger, puis nommer, puis afficher.

**526 avant 527** — le mailer rend des gabarits ; sans eux il ne compile pas.

**515 avant 527**, et c'est la seule dépendance qui remonte vers l'unité 1 :
c'est elle qui a posé le trait `ISurveyMailer` et son implémentation provisoire,
que la 527 remplace.

**526 ne dépend de rien**, et peut donc partir en parallèle du chemin public. Un
gabarit d'e-mail se rend et se relit seul — c'est même la seule carte de cette
unité qu'on peut vérifier à l'œil avant qu'aucun code ne l'appelle.

## Deux choix de découpage

### La page ne se livre pas en deux moitiés

La 525 porte la route, le handler, les trois gabarits, la feuille et le test du
routeur. C'est une grosse carte, et la couper aurait produit une moitié qui ne
compile pas : un handler sans gabarit n'a rien à rendre, un gabarit sans handler
n'est atteint par personne.

Le découpage naturel — « la route » puis « la page » — aurait en plus dispersé le
verrou : le test qui prouve que la route n'est pas sous `require_auth` n'a de
sens qu'une fois la page rendue, sinon il n'atteste que d'un `404` non protégé.

### Les gabarits d'e-mail se livrent avant leur expédition

L'inverse — un mailer d'abord, ses gabarits ensuite — aurait fait relire des
e-mails déjà envoyables. Un gabarit d'e-mail est précisément la chose qu'on veut
regarder dans un vrai client avant qu'un envoi ne parte : la 526 s'arrête à ce
point, et sa checklist le demande.

## Ce que ces cartes achèvent

À la fin de la 528, **le critère « Terminé quand » de l'épic est atteignable** :
un organisateur ouvre une campagne, les e-mails partent, les coachs répondent
d'un clic, et le tirage écrit ses rencontres au calendrier — sans qu'une seule
présence ait été saisie à la main.

Reste alors l'unité `encart-competition`, qui n'est pas sur ce chemin : c'est un
second chemin de réponse, pour quand l'e-mail se perd.
