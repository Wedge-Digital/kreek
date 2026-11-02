# E23 — Le temps réel : connexions, présence et notifications

**État :** `en_cours` — 6 cartes, 642 à 647 ; 4 faites (642 à 645). Découpage validé le 2026-11-01 ; cadrage et décisions le 2026-11-02.
**Cadrage :** `docs/specs/saisie-mobile/E23-cadrage.md`. Née le 2026-11-01 des maquettes
mobiles. Dépend de E21 (le journal des deux camps). **Ordre décidé le 2026-11-01 :
E21, E23, puis E22** — le mode « Je saisis tout » en profite déjà : la lecture
seule de l'autre coach devient un suivi en direct.

## La fonction

L'application n'a aucun canal du serveur vers le navigateur en production : ni
SSE, ni websocket. Seul le suivi de `/live` interroge le serveur toutes les
15 s (`live-match.html:56`, carte 636). Le toast existe (`app-layout.html`,
`kreekToast`), mais seule la réponse HTTP de l'utilisateur lui-même le
déclenche. Un coach ne voit donc pas ce que saisit l'autre sans attendre ou
recharger, et n'est prévenu de rien.

L'épic pose ce canal, et la question qui va avec : **plusieurs matchs se
déroulent en parallèle — à qui envoyer quoi ?**

Le principe retenu : **les destinataires se déduisent du domaine, la présence
ne décide que de la livraison.** Les coachs concernés par un match, c'est le
domaine qui le dit : le coach de chacune des deux équipes, bornées à l'espace.
La résolution des destinataires de E02 (carte 337) n'est pas réutilisée — elle
parle d'invités et d'inscrits d'une saison —, seul son patron l'est (port et
adapter). Savoir s'ils sont connectés, et sur quel écran, c'est le registre des
connexions.

Deux usages :

- **les notifications** — « Saisie du match démarrée par Morgane », « Touchdown :
  Griff #9 (London Rats) — par Morgane », la sélection des coups de pouce
  validée avec ce qui a été pris : un toast, où que soit le coach dans
  l'application ;
- **le direct** — le journal et le score de `/live` se mettent à jour chez qui
  les regarde. L'attente de l'outsider et l'état des validations des camps
  n'existent pas en « Je saisis tout » : ils viendront avec E22, branchés sur
  ce canal.

**Les notifications sont éphémères** (décision du 2026-11-01) : un coach
connecté reçoit le toast ; déconnecté, il ne reçoit rien et rien n'est gardé.
Pas de cloche, pas d'historique.

## Les cartes

| # | Intitulé | Apport |
|---|---|---|
| 642 | Le canal SSE et le registre des connexions | un flux par **onglet** ; registre utilisateur → connexions ; battement de cœur à 15 s, nettoyage à la déconnexion, reconnexion du navigateur ; journalisé sous `kreek::`. **La carte porte la configuration nginx à appliquer avant livraison** — bloc `location`, `http2`, et l'en-tête `X-Accel-Buffering: no` posé par l'application |
| 643 | Les faits de saisie sortent du BC | `emettre()` là où les faits n'étaient pas émis, puis app events avec leur auteur, publiés par le publisher de `match_report` : démarrage, action (MVP compris), retrait, sélection des coups de pouce validée, coups de pouce annulés |
| 644 | La présence sur un match | la page d'un match déclare sa présence : on sait qui le suit et qui le saisit (« Morgane saisit ») |
| 645 | Le routage des notifications | destinataires résolus par port, croisés avec les connectés ; le toast global gagne un sous-titre et un lien |
| 646 | Le direct sur la page d'un match | le journal et le score de `/live` se rafraîchissent sur signal ; le `every 15s` disparaît |
| 647 | Les tests du temps réel | le scénario du « Terminé quand » : trois matchs en parallèle, plusieurs navigateurs. Chaque carte précédente porte ses propres tests unitaires et e2e |

## Ce qui commande l'ordre

**642 et 643 en parallèle** (décision du 2026-11-02) : faire sortir les faits
du BC ne dépend pas du canal. Puis **644 et 645**, la présence demandant le
canal, le routage demandant le canal et les faits. Puis **646**, qui a besoin
des trois. Puis **647**.

**Posé par écrit dans la fiche technique** : le registre vit en mémoire,
comme les sessions aujourd'hui (`DashMapStore`, carte 491). C'est juste tant
qu'il n'y a qu'une instance ; à plusieurs, il faudra un relais —
`LISTEN/NOTIFY` de Postgres, puisque Redis est écarté.

Le piège à éviter : les app events ne sortent du BC que par le publisher
(`CLAUDE.md`, « Émission des app events »), et le canal SSE est une
infrastructure transverse qui ne lit aucune table de BC.

## Conséquences sur la production

**Le canal** ne lit ni n'écrit aucune donnée persistante : le registre vit en
mémoire, les app events transitent par le bus sans être stockés. **La 643, en
revanche, écrit dans `event_log`** : les faits de saisie, désormais émis sur le
bus interne, y sont enregistrés par `event_log_feeder` — quelques dizaines de
lignes par match, dans une table sans lecteur. Les autres effets portent sur
l'exploitation :

- **nginx**, devant le binaire, n'est pas décrit dans le dépôt. Un flux SSE est
  une connexion longue : un tampon de réponse actif ou un délai d'inactivité
  trop court l'empêchent d'arriver ou le coupent en boucle, et sans `http2` le
  navigateur plafonne à six connexions par origine. La 642 fixe la
  configuration, à appliquer avant la livraison ;
- une connexion ouverte par onglet authentifié — négligeable à l'échelle d'une
  ligue amateur ;
- un redéploiement coupe les flux ; les sessions en mémoire étant perdues
  (carte 491), le coach est de toute façon déconnecté, et le flux ne se
  reconnecte pas ;
- l'enum partagée des app events gagne des variants. **Aucun listener des
  autres BCs n'est exhaustif** : ils filtrent par `let … else` ou par un joker,
  rien ne casse et le compilateur n'exige rien d'eux ;
- le toast global vit dans le layout commun : les toasts existants de
  `team_creation` et du calendrier doivent rester identiques.

## Ce que l'épic ne couvre pas

- **Une boîte de notifications**, une cloche, des notifications gardées pour
  plus tard : exclu, les notifications sont éphémères.
- **Les notifications e-mail** : E02, close.
- **Les badges de la barre d'onglets** : E24.
- **« Camp validé », l'attente de l'outsider, l'état des validations** : E22.
- **Le middleware CSRF** que décrit `CLAUDE.md` et qui n'existe pas : carte
  648, sans épic.

## Terminé quand

Trois matchs sont saisis en parallèle dans le même espace : chaque coach ne
reçoit, en toast, que les notifications de ses propres matchs, et le journal
d'un match se met à jour chez qui le regarde sans recharger la page. Un coach
qui ferme son onglet disparaît du registre.
