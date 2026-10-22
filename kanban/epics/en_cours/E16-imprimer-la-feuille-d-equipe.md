# E16 — Imprimer la feuille d'équipe

**État :** `en_cours` — 16/16 cartes livrées. Le critère « Terminé quand » est
vérifié sous Chrome et Edge, pas sous Firefox : son PDF ajoute une page blanche
après la dernière feuille (cf. README de la spec, hors périmètre). L'épic reste
ouverte tant que ce défaut n'est pas tranché — corrigé, ou accepté par écrit.
**Conception :** `docs/specs/impression-feuille-equipe/` — workflow feature,
phases 1 à 8.
**Maquette :** `assets/rawpages/html/app-team-print-modal.html`

## La fonction

Le bouton « Imprimer en PDF » de la fiche d'équipe appelle `window.print()` sur
la page web : sidebar comprise, sans feuille de style d'impression, et parfois
avant que les joueurs aient fini de charger.

L'épic le remplace par une **page « Imprimer »** : le coach compose ce qu'il
imprime — son équipe, son prochain adversaire ou une autre équipe de l'espace,
l'annexe des compétences, les coups de pouce et star players disponibles, une
feuille de match à remplir à la main — et un aperçu A4 se met à jour à chaque
option. « Imprimer » sort les feuilles seules ; le navigateur propose
« Enregistrer en PDF ».

## Les cartes

| # | Intitulé | PR |
|---|---|---|
| **594** | La seconde équipe d'une URL est contrôlée par `space_scope` | 1 |
| **595** | La page « Imprimer » existe | 1 |
| **596** | L'aperçu assemble les feuilles et s'imprime | 1 |
| **597** | L'en-tête de fiche imprimable | 1 |
| **598** | Le tableau des joueurs imprimable | 1 |
| **599** | Imprimer avec le prochain adversaire | 1 |
| **600** | Imprimer avec une autre équipe de l'espace | 1 |
| **601** | Les tests e2e de la page « Imprimer » | 1 |
| **602** | Le favori et la petite monnaie deviennent des fonctions pures | 2 |
| **603** | La disponibilité des star players rejoint le domaine | 2 |
| **604** | `match_reminders_fr.json`, premier fichier optionnel du corpus | 2 |
| **605** | `match_report` lit le catalogue de références | 2 |
| **606** | L'annexe des compétences | 2 |
| **607** | Les coups de pouce et les star players disponibles | 2 |
| **608** | La feuille de match imprimable | 2 |
| **609** | Les tests e2e des annexes et de la feuille de match | 2 |

## Ce qui commande l'ordre

**Deux PR** étaient prévues, chacune utilisable seule et verte sur `lint`,
`check-arch`, `test` et `e2e` : la page et les fiches d'équipe, puis les annexes
et la feuille de match. **La livraison s'est faite en une seule PR**, découpée en
commits par lot : les deux moitiés ont été testées ensemble, et les séparer
après coup n'apportait rien à la relecture.

La **594 vient d'abord** : elle pose le contrôle de la seconde équipe avant
qu'aucune route ne la reçoive. Puis la page (595), l'aperçu (596), ses deux
fragments (597, 598), et les deux façons d'ajouter une équipe (599, 600).

Dans la PR 2, le domaine et les ports d'abord (602 à 605), indépendants entre
eux sauf la 605, qui lit ce que livrent la 603 et la 604. Les trois fragments
(606 à 608) ne dépendent que d'eux et de l'aperçu.

## Ce que l'épic ne couvre pas

- La fuite de `team-selection/json`, qui ne vérifie pas l'espace de la saison.
- Le filtre `availableForRosters` absent à l'engagement d'un star player.
- La fiche statique de `team_creation` (`/app/{space_id}/team/{team_id}/detail`).
- Les prix du staff codés en dur dans la fiche d'équipe.
- Les vrais textes de `match_reminders_fr.json` : le catalogue de production
  n'est pas dans le dépôt.
- Un PDF produit par le serveur : décision prise, c'est le navigateur.

## Terminé quand

Depuis la fiche d'une équipe prête à jouer de seize joueurs, un coach connecté
imprime sa fiche, celle de son prochain adversaire, l'annexe des compétences,
les coups de pouce et star players disponibles et une feuille de match ; le PDF
que propose le navigateur compte **exactement** les feuilles de l'aperçu,
chacune sur une page A4, sans sidebar ni options, et sans une ligne coupée.
