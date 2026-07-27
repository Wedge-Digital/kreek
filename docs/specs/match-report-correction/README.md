# Match report correction — Spec index

Permettre la **correction d'un rapport de match déjà publié**, en annulant ses
effets sur les autres BCs puis en les réappliquant à la re-publication.

Stratégie retenue : **dépublication + rejeu**, et non propagation de deltas.
Nouvelle arête `Published → ReadyToPublish` dans la machine à états de
`match_report`, émettant un app event de compensation symétrique de la
publication. Chaque BC impacté défait l'impact du dernier match ; la
re-publication rejoue ensuite le flux nominal **inchangé**.

## Pages

| Page | Front | Back | DTOs | Use cases | Domaine | Intégration | Cartes |
|---|---|---|---|---|---|---|---|
| Recap (`recap.html`) | ✅ | ✅ | ✅ | ✅ | ✅ | | |

Maquette : `assets/rawpages/html/app-match-report-recap-correction.html` (5 états).

## Pourquoi pas une propagation par deltas

Les bonus de classement sont des **effets de seuil** (`casualties > seuil`,
`td >= seuil`, `td_conceded <= seuil`) : la variation ne détermine pas le
résultat, seule la valeur absolue le fait. Passer de 2 à 3 sorties vaut 0 ou 1
point selon la configuration de la compétition. Une propagation par delta est
donc **fausse par construction** sur le ranking.

À l'inverse, la dépublication offre un chemin de correction **unique et
indépendant de ce qui a été modifié** : défaire l'impact du dernier match, puis
rejouer le flux nominal existant. Le flux de publication actuel n'est pas
touché — on ajoute une compensation symétrique, pas une variante de propagation.

## Règles métier (validées)

1. Correction possible uniquement si les **2 équipes** sont en phase
   `GamePhase::PlayerImprovement`.
2. Correction impossible si un SPP a été dépensé sur l'un des **2** effectifs.
3. Le message de blocage **nomme l'équipe** qui bloque — le port remonte une
   raison typée, jamais un booléen.
4. Droits de correction = droits de publication (admin d'espace, admin de
   compétition, ou coach de l'une des 2 équipes).
5. Pas de motif de correction obligatoire.
6. Les **équipes** du rapport (step 1) ne sont pas modifiables. Modifiables :
   actions, inducements, gains et fan mods.
7. Un rapport dépublié jamais republié **reste en l'état** : effets annulés,
   match absent du classement. Aucun mécanisme de relance ni d'expiration.
8. Le nombre de corrections successives n'est **pas limité**. Corollaire :
   compensation et rejeu doivent être rigoureusement symétriques — deux
   corrections successives menant au même contenu doivent converger vers le
   même état qu'une seule.
9. Le garde-fou est **revérifié côté serveur au POST**. L'affichage du bouton
   n'est qu'un indice : l'adversaire peut invalider la correction à tout
   instant, y compris pendant que la page est ouverte.
10. Les notifications (prévenir l'adversaire qu'un match qu'il a joué vient
    d'être corrigé) sont **hors scope**, à traiter dans un second temps.
11. **Échec partiel de la compensation** — le bus étant best-effort, une
    compensation peut réussir dans un BC et échouer dans un autre. Posture
    retenue : on l'accepte, en s'appuyant sur l'idempotence de chaque
    compensation et sur une resynchronisation manuelle si nécessaire. Ni
    ordonnancement par criticité, ni compensation synchrone dans le use case —
    cette dernière violerait la souveraineté des données entre BCs. C'est le
    seul endroit où la feature peut laisser la base incohérente, et c'est assumé.
12. **Échec d'un port du garde-fou** — si `is_team_in_player_improvement` ou
    `has_spent_spp_since_match` ne peut pas répondre, on **échoue fermé** : le
    rapport est déclaré non corrigeable. Un garde-fou échouant ouvert
    autoriserait une correction qui aurait dû être refusée — c'est la direction
    dangereuse. Échouer fermé empêche temporairement une correction légitime :
    gênant, jamais destructeur.

## Le garde-fou « à chaud »

Les deux conditions des règles 1 et 2 sont cumulatives et de nature différente :

**Condition 1 — structurelle, offerte par la machine à états.** Toutes les
actions qui rendraient la correction impossible exigent une phase *ultérieure* à
`PlayerImprovement` : `buy_staff` exige `Recruitment`, `dismiss_staff` exige
`Dismissals`, `start_match_reporting` (match suivant) exige `ReadyToPlay`. Donc
« les 2 équipes encore en `PlayerImprovement` » implique gratuitement : aucun
match ultérieur, aucun mouvement de trésorerie depuis le gain, aucun mouvement
d'effectif, `dedicated_fans` intact.

**Condition 2 — nécessaire en plus.** `Player::purchase_skill()` et
`increase_stat()` ne vérifient que `spp_remaining()`, jamais la phase de
l'équipe (le BC `players` ne la connaît pas). Un coach peut donc dépenser ses
SPP sans quitter `PlayerImprovement`.

Ces deux garde-fous sont des **vérifications bloquantes** → ports (consultation
synchrone), pas de cache alimenté par event (cf. CLAUDE.md, « Consultation vs
propagation d'effet entre BCs »).

## Ce que la restriction « à chaud » simplifie

- **Ranking** : les 2 lignes du match sont les dernières de chaque équipe → un
  `DELETE` puis rejeu, aucun recalcul en cascade.
- **Teams / players** : aucune migration d'event. La valeur de `dedicated_fans`
  d'avant le match — perdue car `PostMatchSequenceStarted` ne stocke que la
  valeur post-clamp — se reconstruit en **état dérivé** : `apply()` voit
  `self.dedicated_fans` avant de l'écraser. Même mécanique côté `players` pour
  l'instantané « contribution du dernier match ».
- **Match report** : le retour arrière existe déjà. Tous les use cases d'édition
  font `MatchReportState::ReadyToPublish(rtp) => rtp.into_pre_match()`. Il ne
  manque qu'une arête `Published → ReadyToPublish`.

13. `rehydrate()` supporte **N alternances** publier / dépublier (corollaire de
    la règle 8). Satisfait par construction — `rehydrate` est un `fold` — donc à
    couvrir par un test, pas par du code.
14. Les **fans se restaurent par instantané, jamais par soustraction**.
    `PostMatchSequenceStarted` ne stocke que la valeur post-clamp : si `+2` a été
    écrêté à 20, retrancher 2 donnerait 18 au lieu de 20.
15. Le **statut de participation du joueur se restaure** aussi. Un joueur
    `MissingNextGame` avant le match a été rendu `Available` par
    `MatchConcluded` ; la compensation doit l'y remettre. Voir la réserve du
    point ouvert dans `recap/06-domaine.md`.
16. **Équipe retirée ou dissoute** (`TeamDismissed` → `game_phase = None`) :
    l'imprécision du libellé est acceptée. Le blocage reste correct sur le fond,
    et une équipe dissoute rend la correction sans objet.

## Dettes préexistantes à traiter dans cette feature

1. **`post_publish` ne vérifie aucune autorisation** (`recap_controller.rs`, POST
   publish) : il contrôle uniquement qu'un utilisateur est connecté, contrairement
   au GET qui appelle `is_authorized()`. Tout utilisateur connaissant un
   `match_report_id` peut publier le rapport d'autrui. La règle 4 alignant les
   droits de correction sur ceux de la publication, cette dette doit être réglée
   **en amont**, sinon la correction en hérite.
2. **`ranking_lines` n'a aucune contrainte d'unicité** sur
   `(match_report_id, team_id)` — à ajouter pour rendre le rejeu sûr et détecter
   les doublons.
3. **`resolve_pairing_id`** (competitions) recréerait un **second pairing** pour
   un rapport manuel si la re-publication repassait par le chemin de création.

## Risque transverse

Le bus est un `tokio::broadcast` best-effort, **sans outbox**. Une dépublication
dont la compensation se perd (crash, `Lagged(n)`) laisse un état *pire* qu'avant :
rapport redevenu éditable, mais impacts toujours appliqués en base,
silencieusement.

Traitement retenu : rendre chaque compensation **idempotente et
re-déclenchable**, plutôt que d'introduire une outbox. Cohérent avec la règle 8,
qui exige déjà la convergence.
