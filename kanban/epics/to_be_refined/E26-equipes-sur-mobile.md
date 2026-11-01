# E26 — Les équipes sur mobile : recherche et fiches

**État :** `to_be_refined` — 5 cartes à écrire. Découpage validé le 2026-11-01. Née le 2026-11-01 des maquettes
mobiles. Indépendante des épics de saisie ; s'appuie sur E06 pour le bilan
V-N-D.

**Maquettes :** `app-mobile-teams.html`, `app-mobile-teams-search.html`,
`app-mobile-team.html`, `app-mobile-team-player.html`.

## La fonction

**On ne trouve pas une équipe.** Il n'existe aucune recherche d'équipe ; la
recherche de coachs de `spaces` sert aux formulaires, pas à la consultation. La
cible : un seul champ, « Nom d'équipe ou de coach », sur les équipes **inscrites
dans une saison en cours** de l'espace ; un coach trouvé ramène ses équipes.
`team_proj` porte déjà `coach_name` : **tout se cherche dans `teams`**, sans
`spaces` (décisions du 2026-11-01).

**On consulte mal une équipe.** L'effectif est un tableau de quatorze colonnes
qui défile en largeur ; la fiche joueur, déjà une page complète, n'a pas de mise
en page mobile et n'affiche pas les blessures persistantes en tête.

L'ordinateur ne change pas.

**Un gabarit par taille quand la structure diffère** (décision du 2026-11-01) :
l'effectif en tableau et l'effectif en cartes ne sont pas le même composant.
Le widget reçoit la taille cible, calculée par le navigateur avec la même
requête média que le CSS (`matchMedia('(max-width: 768px)')`), et rend l'un ou
l'autre gabarit ; route, contrôleur, use case et view model restent communs.
Pas de détection par User-Agent. Quand seule la disposition change — en-tête de
la fiche, fiche joueur —, le CSS suffit.

## Les cartes

| # | Intitulé | Apport |
|---|---|---|
| — | Chercher une équipe dans l'espace | un widget de recherche dans `teams`, sur `team_proj` : nom d'équipe ou de coach, équipes inscrites dans une saison en cours ; les équipes trouvées par leur nom, puis les coachs trouvés et leurs équipes |
| — | La page Équipes | « Mes équipes » (widget existant), puis la recherche ; comme « Mes matchs », une route que seule la barre mobile référence |
| — | L'effectif mobile, son propre gabarit | le widget des joueurs de `players` rend une carte par joueur sous 768 px — caractéristiques, compétences, SPP, absence, blessures persistantes (nombre) —, le tableau d'aujourd'hui au-dessus. Premier usage du gabarit par taille : la carte pose le mécanisme — un script global qui ajoute la taille aux requêtes des widgets concernés, et leur rechargement par un événement sur `body` quand la fenêtre franchit 768 px |
| — | Les chiffres clés de la fiche équipe | par CSS — l'en-tête est rendu avec la page : VE, trésorerie, fans, bilan V-N-D sur mobile ; se livre sans le bilan, puis branche le widget de la carte `match-01` de E06 |
| — | La fiche joueur sur mobile | par CSS : mise en page mobile ; blessures persistantes en tête **sur les deux tailles** ; bouton « Faire évoluer » vers le panneau de dépense en phase d'amélioration, **sur mobile seulement** — sur ordinateur, le panneau est déjà à côté (décisions du 2026-11-01) |

## Ce qui commande l'ordre

La recherche, puis la page Équipes qui la compose. Les trois cartes de fiche
sont indépendantes entre elles et de la recherche ; l'effectif mobile pose le
mécanisme du gabarit par taille, à faire tôt — d'autres épics pourraient s'en
servir. Les chiffres clés attendent
la carte `match-01` de E06 pour le bilan V-N-D, ou se livrent d'abord sans lui.

## Conséquences sur les données de production

**Lecture seule** : rien n'est écrit ni migré.

- `coach_name` est une copie dans `team_proj` : si un coach pouvait changer de
  nom, la recherche trouverait l'ancien — la première carte vérifie que ce
  changement n'existe pas ;
- une dizaine de tests e2e lisent le tableau des joueurs
  (`test_feuille_caracteristiques`, `test_feuille_blessures_persistantes`,
  `test_roster_edition`…) : ils tournent en taille ordinateur et voient le
  gabarit inchangé. Un second balisage caché aurait doublé chaque texte et fait
  tomber leurs sélecteurs en mode strict — c'est ce que le gabarit par taille
  évite ;
- `test_my_teams_page`, `test_player_detail`, `test_player_spp_spending` touchent
  les pages retouchées par CSS ;
- **seul changement visible sur ordinateur** : les blessures persistantes en
  tête de la fiche joueur.

## Ce que l'épic ne couvre pas

- **Les onglets Matchs et Trésorerie de la fiche**, et le widget V-N-D : E06.
- **Les évolutions elles-mêmes** — le parcours de dépense des SPP : le bouton
  « Faire évoluer » mène au panneau existant.

## Terminé quand

Sur un téléphone, on tape trois lettres du nom d'un coach et l'on voit ses
équipes ; on ouvre l'une d'elles et l'on lit son effectif sans défilement
horizontal, blessures persistantes comprises ; on ouvre un joueur et l'on voit
ses blessures persistantes en tête de sa fiche.
