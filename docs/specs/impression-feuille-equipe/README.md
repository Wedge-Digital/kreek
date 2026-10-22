# Imprimer la feuille d'équipe — Progression

Le bouton « Imprimer » de la fiche d'équipe imprime aujourd'hui la page web
telle quelle (`window.print()`), sans feuille de style d'impression, et avant
que les widgets HTMX aient fini de charger. Il ouvre désormais une **modale
d'impression** : le coach compose ce qu'il imprime, et un aperçu A4 se met à
jour à chaque option.

## Maquette (Phase 1 ✅)

Validée et commitée (`165d852a`) :

| Écran | Fichier |
|---|---|
| Modale d'impression et ses pages | `assets/rawpages/html/app-team-print-modal.html` |

## Décisions

- **Le PDF est produit par le navigateur.** Le serveur rend une page HTML
  d'impression ; l'aperçu l'affiche réduite, dans une iframe ; « Imprimer »
  ouvre la boîte d'impression, qui propose « Enregistrer en PDF ». Pas de
  Chromium côté serveur, pas de bouton de téléchargement.
- **Aucune règle de jeu dans le code.** Ce que kreek code déjà s'imprime tel
  quel : gains, petite monnaie, mercenaires, deux star players au plus,
  barème SPP par roster. Le reste vient du corpus de références (cf. ci-dessous).
- **« Une autre équipe » reste dans l'espace de mon équipe** : compétition,
  puis équipe.
- **La feuille de match tient sur une page A4**, en portrait comme en paysage.

### Un nouveau fichier optionnel du corpus : `match_reminders_fr.json`

Les rappels du MVP et des fans dévoués, et la liste des météos, sont des règles
de jeu que kreek ne code nulle part. Aucun des neuf fichiers obligatoires du
corpus (`docs/reference-data-schema.md`) ne les porte : ils arrivent dans un
**nouveau fichier**, à trois champs — `mvp` et `fans` (textes), `weather`
(liste de libellés).

- **Optionnel** : il ne rejoint pas les neuf obligatoires. Un corpus qui ne
  l'a pas démarre comme avant ; l'impression sort alors sans rappels du MVP et
  des fans, et la météo devient une ligne libre.
- **Version fictive commitée** dans `assets/references.example/`, format
  documenté dans `docs/reference-data-schema.md`.
- **Le vrai catalogue n'est pas dans le dépôt** (`assets/references/`, ignoré
  par git, sous copyright). Ses vrais textes restent à ajouter par qui le
  détient — **à signaler dans la PR**.
- **Portée : l'instance.** Le corpus vaut pour tous les espaces et toutes les
  compétitions ; deux ligues d'une même instance aux règles de MVP différentes
  ne sont pas prévues.

## Contenu de la modale

| Groupe | Options |
|---|---|
| Équipes | mon équipe seule · avec le prochain adversaire · avec une autre équipe de l'espace |
| Pages en plus | compétences · coups de pouce disponibles · star players disponibles · feuille de match (×2 en option) |
| Mise en page | orientation · densité · encre · style (Sobre, Officielle, Terrain) |
| Sur les fiches | joueurs indisponibles (afficher, barrer, masquer) · lignes vides jusqu'à 16 |

## Hors périmètre

- La route `team-selection/json` ne vérifie pas que la saison appartient à
  l'espace de l'URL.
- Le serveur n'applique pas `availableForRosters` à l'engagement d'un star
  player.
- `team_creation` sert encore une fiche d'équipe **entièrement statique**
  (`/app/{space_id}/team/{team_id}/detail`, carte 26) : roster en dur, boutons
  sans action — dont un « Imprimer en PDF » —, aucun lien n'y mène. Nettoyage à
  faire à part, après vérification de ses usages.
- La fiche d'équipe code en dur le prix du staff (`StaffVm` : apothicaire 50,
  assistant 10, pom-pom 10) alors que la VE les lit dans le catalogue du roster.
  L'impression prend le catalogue ; la fiche reste à aligner.
- **Firefox imprime une page blanche après la dernière feuille.** Chromium (Chrome,
  Edge) sort exactement les feuilles de l'aperçu, vérifié par le test e2e E6.
  Deux correctifs n'ont pas suffi (feuilles de 2 px sous la hauteur du papier,
  `html` et `body` en hauteur automatique à l'impression) ; le défaut n'a pas été
  reproduit hors de la boîte d'impression de Firefox.

## Écarts relevés à l'implémentation

La conception (phases 2 à 7) a tenu ; ces points ont été ajustés en codant, et
les fichiers de phase ou les cartes concernés le disent.

- **Masquer le chrome à l'impression** : par `layout-app.css`
  (`.app-layout:has(.team-print)`), et non par un bloc `body_class` — la
  navigation htmx ne recharge jamais `body` (`03-back.md`, décision 4).
- **L'aperçu se recharge par le script de la page** (`htmx.ajax`), et non par
  un `hx-get` figé : l'URL change avec la seconde équipe, qui va dans le chemin.
- **Fragments multi-pages** : l'annexe des compétences, les coups de pouce et
  la feuille de match rendent **eux-mêmes** leurs feuilles (`print-slot` ›
  `print-frame` › `print-sheet`) ; l'hôte ne pose que l'emplacement. Seule la
  fiche d'équipe est une feuille ouverte par l'hôte. Ces classes sont le
  contrat entre la page de `teams` et les fragments des autres BCs.
- **Date et numéro de page** : posés par le script de la page sur toutes les
  feuilles, quel que soit le BC qui les a rendues.
- **Le tableau des joueurs ne reçoit pas la densité** : elle ne joue qu'en CSS.
- **Les règles d'achat rappelées lisent les constantes du code** : le domaine
  de `match_report` expose `MAX_TREASURY_TOP_UP`, `MAX_STAR_PLAYERS` et
  `MAX_MERCENARIES`, et `init_temp_players_use_case` `JOURNEYMEN_FILL_TO` —
  des littéraux nommés, valeurs inchangées.
- **`NextOpponent` porte un `Venue`** et non un booléen (axe 6).
- **Les méthodes ajoutées aux ports existants** de `match_report`
  (`list_alignable_players`, `find_team_season`) ont un corps par défaut
  neutre : les cinq doublures de test de ces ports n'ont pas à changer.

## Progression par page

La maquette montre une modale ; l'application en fait une **page dédiée**
(cf. `team-print/02-front.md`).

| Page | Front | Back | DTOs | Use cases | Domaine | Intégration | Cartes |
|---|---|---|---|---|---|---|---|
| Page « Imprimer » | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ **594-609** |
