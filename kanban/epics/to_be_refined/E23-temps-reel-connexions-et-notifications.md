# E23 — Le temps réel : connexions, présence et notifications

**État :** `to_be_refined` — 6 cartes à écrire. Découpage validé le 2026-11-01. Née le 2026-11-01 des maquettes
mobiles. Dépend de E21 (le journal des deux camps). **Ordre décidé le 2026-11-01 :
E21, E23, puis E22** — le mode « Je saisis tout » en profite déjà : la lecture
seule de l'autre coach devient un suivi en direct.

**Conception à écrire avant les cartes** : une fiche technique — connexions,
routage, panne, observabilité. Ce n'est pas un écran, le workflow feature page
par page ne s'y applique pas.

## La fonction

L'application n'a aucun canal du serveur vers le navigateur : ni SSE, ni
websocket, ni polling. Le toast existe (`app-layout.html`, `kreekToast`), mais
seule la réponse HTTP de l'utilisateur lui-même le déclenche. Un coach ne voit
donc pas ce que saisit l'autre sans recharger, et n'est prévenu de rien.

L'épic pose ce canal, et la question qui va avec : **plusieurs matchs se
déroulent en parallèle — à qui envoyer quoi ?**

Le principe retenu : **les destinataires se déduisent du domaine, la présence
ne décide que de la livraison.** Les coachs concernés par un match, c'est le
domaine qui le dit (les deux équipes, bornées à l'espace — comme la résolution
des destinataires de E02, carte 337). Savoir s'ils sont connectés, et sur quel
écran, c'est le registre des connexions.

Deux usages :

- **les notifications** — « Saisie du match démarrée par Morgane », « Touchdown
  du 9 des London Rats saisi par Morgane » : un toast, où que soit le coach dans
  l'application ;
- **le direct** — le journal d'un match se met à jour chez qui le regarde ;
  l'outsider qui attend le favori, le coach qui attend la validation de l'autre
  voient l'état changer.

**Les notifications sont éphémères** (décision du 2026-11-01) : un coach
connecté reçoit le toast ; déconnecté, il ne reçoit rien et rien n'est gardé.
Pas de cloche, pas d'historique.

## Les cartes

| # | Intitulé | Apport |
|---|---|---|
| — | Le canal SSE et le registre des connexions | un flux par session ; registre utilisateur → connexions (plusieurs onglets) ; battement de cœur, nettoyage à la déconnexion, reconnexion du navigateur ; journalisé sous `kreek::`. **La carte spécifie la configuration nginx à adopter** pour la route du flux (tampon de réponse, délais d'inactivité) |
| — | La présence sur un match | la page d'un match s'abonne à ce match : on sait qui le suit et qui le saisit (« Morgane saisit ») |
| — | Les faits de saisie sortent du BC | app events avec leur auteur, publiés par le publisher de `match_report` : démarrage, action, MVP, achats finis, camp validé, coups de pouce annulés |
| — | Le routage des notifications | destinataires résolus par port, croisés avec les connectés ; le toast global gagne un sous-titre et un lien |
| — | Le direct sur la page d'un match | le journal, l'attente de l'outsider et l'état des validations se rafraîchissent sur événement |
| — | Les tests du temps réel | unitaires sur le registre et le routage ; e2e à deux navigateurs, dont trois matchs en parallèle |

## Ce qui commande l'ordre

Le canal et le registre d'abord, puis deux branches indépendantes : la présence
et le direct d'un côté, les app events et le routage des notifications de
l'autre.

**À poser par écrit dans la fiche technique** : le registre vit en mémoire,
comme les sessions aujourd'hui (`DashMapStore`, carte 491). C'est juste tant
qu'il n'y a qu'une instance ; à plusieurs, il faudra un relais —
`LISTEN/NOTIFY` de Postgres, puisque Redis est écarté.

Le piège à éviter : les app events ne sortent du BC que par le publisher
(`CLAUDE.md`, « Émission des app events »), et le canal SSE est une
infrastructure transverse qui ne lit aucune table de BC.

## Conséquences sur la production

**Aucune donnée persistante** n'est lue ni écrite : le registre vit en mémoire,
les app events transitent par le bus sans être stockés. Les effets portent sur
l'exploitation :

- **nginx**, devant le binaire, n'est pas décrit dans le dépôt. Un flux SSE est
  une connexion longue : un tampon de réponse actif ou un délai d'inactivité
  trop court l'empêchent d'arriver ou le coupent en boucle. La première carte
  fixe la configuration de la route du flux, à appliquer avant la livraison ;
- une connexion ouverte par coach connecté — négligeable à l'échelle d'une
  ligue amateur ;
- un redéploiement coupe les flux ; le navigateur se reconnecte, mais les
  sessions en mémoire étant perdues (carte 491), le coach est de toute façon
  déconnecté ;
- l'enum partagée des app events gagne des variants : les listeners des autres
  BCs qui la parcourent devront les nommer — le compilateur l'exige ;
- le toast global vit dans le layout commun : les toasts existants de
  `team_creation` doivent rester identiques.

## Ce que l'épic ne couvre pas

- **Une boîte de notifications**, une cloche, des notifications gardées pour
  plus tard : exclu, les notifications sont éphémères.
- **Les notifications e-mail** : E02, close.
- **Les badges de la barre d'onglets** : E24.

## Terminé quand

Trois matchs sont saisis en parallèle dans le même espace : chaque coach ne
reçoit, en toast, que les notifications de ses propres matchs, et le journal
d'un match se met à jour chez qui le regarde sans recharger la page. Un coach
qui ferme son onglet disparaît du registre.
