# Phase 2 — Architecture front — player-detail

Maquette de référence : `assets/rawpages/html/app-player-detail-readonly.html`,
mode `customise`.

## Le slot `#pd-right-panel`

La colonne droite de la fiche joueur est un **slot à occupant unique**, déjà
partagé par deux widgets qui se remplacent en `outerHTML` sur le même id :

- `evolution-journal` — le journal des évolutions, occupant par défaut ;
- `spp-spending` — le panneau de dépense de SPP, atteint depuis le journal.

Le mode customisation en devient le **troisième occupant**, et non un mode du
journal. C'est ce que montre la maquette — le panneau bascule entièrement — et
ça réutilise un mécanisme éprouvé.

Répéter l'id du conteneur dans le fragment est ici **correct** : le swap est un
`outerHTML`, le fragment *est* le conteneur. L'interdiction du CLAUDE.md vise
les swaps `innerHTML`, où l'id se retrouverait en double.

## Widgets

| Widget | BC | Endpoint | Trigger | Émet | Mode |
|---|---|---|---|---|---|
| `evolution-journal` *(existant, inchangé)* | players | `GET .../widgets/evolution-journal` | `load` (page) ; retour depuis customisation | — | lecture |
| `spp-spending` *(existant, inchangé)* | players | `GET .../widgets/spp-spending` | clic « Activer la dépense de SPP » | — | mutation |
| `player-customisation` *(nouveau)* | players | `GET .../widgets/customisation` | clic « ✏️ Customiser » (en-tête) ; `load` de la page si un panier existe **et** que l'utilisateur a le droit | — | mutation |

Le déclencheur vit dans l'en-tête de la page hôte, comme dans la maquette. La
page possède le slot : elle le cible directement, exactement comme le fait déjà
`right_panel_widget_url` au chargement. Aucun widget n'en référence un autre.

**Le bouton n'est rendu que si le droit existe.** Un coach ne doit pas le voir :
la condition est serveur, pas une classe CSS masquée.

## Le panier vit côté serveur

Décision prise en révision de cette phase, après examen des deux précédents du
projet.

`teams` fait vivre les paniers de recrutement et de renvois dans
`teams__phase_baskets` : lignes ajoutées par POST unitaires, validation en
bloc, et surtout **hydratation contre l'état courant** à chaque affichage — un
brouillon de dix minutes est évalué contre les prix et l'effectif
d'aujourd'hui. `players` n'a aucun panier : la dépense de SPP engage
immédiatement, chaque geste répondant `HX-Refresh: true`.

Un panier front n'aurait de précédent nulle part. Et l'argument décisif n'est
pas l'harmonisation mais la nature des règles écrites en phase 1 — bornes de
caractéristiques, doublon de compétence, prix ≥ 0, plafond de SPP : **toutes se
jugent contre l'état courant du joueur, panier compris**. Un panier front ne
peut que pré-vérifier ; la vérité reste serveur.

Corollaire heureux : le « refus visible » de la phase 1 devient naturel. Le
refus tombe **au clic**, au moment où le commissaire comprend pourquoi, et non
à l'enregistrement d'un lot dont il faudrait désigner la ligne fautive.

Le panier est **persistant** : un commissaire interrompu retrouve son travail,
et le patron reste identique à celui de `teams`.

### Le mode se déduit du panier, il n'est pas un état du joueur

Le panier porte `player_id` : **« ce joueur a un panier » est exactement « ce
joueur est en cours de customisation »**. Ouvrir le panier — même vide — à
l'entrée dans le mode suffit ; le contrôleur de page choisit l'occupant du slot
en le regardant, et un rechargement complet retombe donc sur le panneau de
customisation sans rien de plus à persister.

**Sous réserve du droit.** Le panier étant propre au joueur et non à son
auteur, il ne suffit pas qu'il existe : le mode ne se rouvre que si
**l'utilisateur qui ouvre la fiche a le droit de customiser**. Sinon c'est la
fiche classique, journal des évolutions compris.

Sans cette condition, un panier laissé ouvert par un commissaire ferait
apparaître le mode administration à un coach ouvrant la même fiche — alors que
la phase 1 pose qu'il ne doit pas même le voir. Le choix de l'occupant du slot
est donc `panier existant **et** droit de customiser`, jamais l'un sans
l'autre.

Aucun état « en cours de customisation » n'est posé sur le joueur, pour deux
raisons.

Le joueur est **event-sourcé** : y poser un état signifierait un événement dans
son flux domaine pour une situation d'interface. Son historique enregistrerait
« quelqu'un a ouvert un panneau » au même rang que « a gagné la compétence
Bloc ». La phase 1 a posé que les événements portent les customisations
elles-mêmes ; un état d'écran n'en est pas une.

Et ce serait une **seconde source de vérité** à côté du panier, avec la
divergence qui finit toujours par arriver : un panier sans état, un état sans
panier.

`teams` procède déjà ainsi — `teams__phase_baskets` est une table de travail,
pas un flux d'événements.

### Concurrence entre commissaires — non traitée, délibérément

Le panier étant propre au **joueur** et non à son auteur, deux commissaires le
partageraient. Le cas n'est pas traité : sa probabilité métier est jugée
quasi nulle.

Conséquence assumée : **le validateur endosse tout le panier**, y compris des
lignes qu'il n'aurait pas ajoutées, et c'est son nom que porte le journal.

C'est un écart connu à la traçabilité nominative de la phase 1. Il est écrit
ici pour qu'il reste un choix, et non une découverte.

## Actions

Chaque action unitaire valide contre `joueur + panier` et **répond par le
panneau re-rendu** — refus compris, affiché sur place. Cible commune :
`hx-target="#pd-right-panel"`, `hx-swap="outerHTML"`.

```
POST .../customisation/skills/add      { skill_id }        → panneau re-rendu
POST .../customisation/stats/add       { stat, crans }     → panneau re-rendu
POST .../customisation/price/adjust    { delta_kpo }       → panneau re-rendu
POST .../customisation/spp/add         { amount }          → panneau re-rendu
POST .../customisation/lines/remove    { line_id }         → panneau re-rendu

POST .../customisation/validate        —  applique tout    → HX-Refresh: true
POST .../customisation/cancel          —  vide le panier   → journal des évolutions
```

`crans` porte le sens en **qualité du joueur** (+1 améliore, −1 dégrade), pas
l'offset brut : la traduction vers la valeur stockée dépend de la
caractéristique et appartient au domaine, seul détenteur de la table de
directions (`apply_increase`).

La validation répond `HX-Refresh: true`, patron déjà utilisé par
`purchase_skill` et `increase_stat` : l'en-tête porte les caractéristiques, la
valeur et les SPP, et n'est pas un widget. Un rechargement complet est le
moyen le plus simple de le remettre à jour sans transformer l'en-tête en widget
— ce qui déborderait de cette fonctionnalité.

## Événements DOM

**Aucun.** Et c'est un choix, pas un oubli : le panneau porte toute
l'interaction, aucun autre widget de la page n'a besoin d'être notifié, et
l'en-tête est remis à jour par le rechargement complet de la validation.

Introduire un événement ici n'aurait aucun abonné.

## Front seul / back

**Front** — bascule d'onglets, recherche au clavier dans la liste de
compétences.

**Back** — tout le reste. Le panier, sa validité, les valeurs effectives, les
aperçus de bornes, le décompte des lignes en attente. Le serveur les connaît
mieux que le navigateur, et c'est lui qui devra de toute façon trancher.

Conséquence à connaître pour la phase 8 : **le JS de la maquette n'est pas
réutilisable**. `pendingChanges`, `queueStat`, `statAfter`, `pendingCrans` y
simulent un panier client qui n'existera pas. La maquette reste valable
visuellement — c'est une maquette de comportement, pas une préfiguration du
code.

## Widgets existants — réutilisables ?

`skill-picker` (BC `references`, `GET /references/roster-lines/skill-picker`)
**n'est pas réutilisable ici**. Il filtre les compétences sur l'accès du poste
(primaire/secondaire) et calcule un coût depuis la matrice de prix — deux
choses que la customisation ignore par définition (« sans passer par les règles
du jeu »).

Sa logique d'exclusion des compétences déjà acquises, en revanche, correspond
exactement à la règle « pas de doublon » : à reprendre, pas à réutiliser.

Le catalogue complet est rendu au chargement du panneau, moins les compétences
déjà possédées, et filtré au clavier côté client. Pas d'endpoint de recherche :
il n'apporterait rien sur une liste de cette taille.

## Règles métier (identifiées phase 2)

- **L'autorisation est vérifiée sur chaque endpoint**, pas seulement au GET du
  widget. Masquer le bouton n'est pas un contrôle d'accès.
- **Le panier est hydraté contre l'état courant du joueur** à chaque
  affichage. Une ligne devenue invalide entre-temps — compétence acquise par
  ailleurs, borne atteinte — doit être signalée à l'affichage, pas seulement à
  la validation.
- **Validation et annulation suppriment le panier** — et pas seulement ses
  lignes : c'est son existence qui commande le mode, un panier vidé mais
  conservé rouvrirait la customisation au prochain chargement.
- **Le validateur endosse l'intégralité du panier**, son nom étant celui que
  retient le journal.

## Points ouverts pour la phase 3

- **Durée de vie du panier.** Celui de `teams` suit une phase de jeu qui a un
  début et une fin. Une session de customisation n'en a pas : rien ne dit quand
  un panier abandonné doit disparaître. Le point est d'autant plus vif que le
  panier commande désormais l'affichage — un panier oublié rouvre le mode
  customisation à chaque visite de la fiche.
- **Un joueur renvoyé pendant qu'un panier le vise** — le panier survit-il, et
  la customisation reste-t-elle applicable ? La phase 1 pose que les
  customisations s'appliquent toujours ; reste à dire ce que « toujours »
  recouvre ici.
