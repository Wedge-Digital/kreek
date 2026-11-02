# E21 — Le mode de saisie et l'après-match en séquence

**État :** `en_cours` — 9 cartes, 631 à 639 ; 6 faites (631, 632, 633, 634, 635, 637). Découpage et cadrage validés
le 2026-11-01.
**Cadrage :** `docs/specs/saisie-mobile/E21-cadrage.md`. Née le 2026-11-01 des maquettes
mobiles. Dépend de E19.

**Maquettes :** `app-mobile-parcours.html`, sections 3 et 3A.

## La fonction

Sur mobile, on a du mal à trouver ce qu'on doit saisir et à voir ce que l'autre
a saisi ; le MVP se saisit au milieu des actions de jeu, alors qu'il conclut le
match.

L'épic introduit le **mode de saisie**, choisi par celui qui démarre le match
et **définitif** : « Je saisis tout » ou « Chacun son équipe ». Elle livre le
premier, qui reste le défaut et le cas courant sur ordinateur ; E22 livre le
second.

**Principe d'implémentation (décision du 2026-11-01)** : pas de routes propres
au mobile. Les routes existantes intègrent un comportement selon le mode du
match. Le serveur ne sait pas sur quel appareil on est ; en « Je saisis tout »,
mobile et ordinateur suivent donc le même parcours.

Ce que l'épic change, partout :

- la carte « Qui saisit ? » sur l'écran de sélection, cochée sur « Je saisis
  tout » ;
- depuis le calendrier, « Commencer » mène droit à l'avant-match, sans écran de
  sélection — « Nouveau match » hors calendrier la garde ;
- en « Je saisis tout », **l'autre coach est en lecture seule** (décision du
  2026-11-01) : il suit, il n'écrit pas ;
- le journal montre les deux camps, chaque action disant qui l'a saisie ;
- le MVP devient **la première étape de l'après-match** : domicile puis
  visiteur, tirage au sort parmi six joueurs désignés ou saisie manuelle. Il
  s'inscrit au journal comme une action `Mvp` au tour 16 — c'est un autre moyen
  de saisir, le domaine des actions ne change pas. Le bouton ⭐ quitte le
  panneau d'actions ;
- cinq étapes : Avant-match · Match · MVP · Gains et fans · Validation.

## Les cartes

| # | Intitulé | Apport |
|---|---|---|
| 631 | Le mode de saisie entre dans l'agrégat | nouvel événement portant le mode et le démarreur, colonne de projection ; tout compte rendu existant se rejoue en « Je saisis tout », son démarreur étant le `confirmed_by` de `SelectionConfirmed` |
| 632 | « Qui saisit ? » au démarrage | la carte sur l'écran de sélection, ordinateur et mobile |
| 633 | Démarrer depuis le calendrier sans sélection | `from_pairing` confirme les équipes de l'appariement et pose le mode ; la confirmation émet toujours `MatchReportConfirmed` |
| 634 | L'autre coach suit en lecture seule | en « Je saisis tout », seul celui qui a démarré écrit — et les admins ; l'autre voit le match sans bouton de saisie. Vaut aussi pour la dépublication et la correction : seul le démarreur corrige |
| 635 | Le journal des deux camps | le widget journal lit les deux camps, avec « saisi par » ; ✕ seulement sur ce qu'on peut supprimer. La projection `match_report_actions` ne stocke pas l'auteur : colonne `recorded_by`, remplie depuis l'event store |
| 636 | Le match en direct | l'écran du journal, « Nouvelle action » demande l'équipe puis ouvre l'étape 3 ou 4 ; ouverte depuis le journal, la page d'action ne garde que « Retour au journal » |
| 637 | L'étape MVP, saisie manuelle | une étape après le match, domicile puis visiteur ; un MVP par camp au tour 16 — règle des nouvelles saisies, jamais du rejeu ; le bouton ⭐ disparaît ; l'indicateur passe à cinq étapes |
| 638 | Le tirage au sort du MVP | six joueurs désignés, tirage côté serveur par un port d'aléa, désignés conservés dans l'événement, résultat définitif |
| 639 | Les tests e2e du nouveau parcours | le parcours complet en « Je saisis tout » ; les tests cassés par une carte sont migrés dans celle-ci — sept fichiers saisissent le MVP comme une action (637), pas quatre |

## Ce qui commande l'ordre

Le mode dans l'agrégat passe en premier : tout le reste en lit la valeur.
« Qui saisit ? » et le démarrage sans sélection suivent, puis la lecture seule,
qui s'appuie sur la garde de E19.

Le journal des deux camps et le match en direct forment une paire ; l'étape MVP
peut partir en parallèle. Le tirage vient après la saisie manuelle, dont il
réutilise l'enregistrement.

**Le coût des tests est à planifier dès la première carte** : presque toute la
suite e2e passe par `match_report_helpers.py`, qui code en dur `/new`, `/step2`,
`/step3|4/actions`, `/step5` et `/recap/publish`. Ce qui reste en « Je saisis
tout » ne doit pas casser ; ce qui change (MVP, démarrage) se migre dans la
carte qui le change.

## Conséquences sur les données de production

| Carte | Données modifiées | Effet sur l'existant |
|---|---|---|
| Mode dans l'agrégat | un événement **ajouté** ; une colonne de projection, défaut « Je saisis tout » | tout compte rendu existant est en « Je saisis tout » ; son démarreur se lit dans `confirmed_by` — les brouillons jamais confirmés n'en ont pas besoin |
| « Qui saisit ? » | aucune | — |
| Démarrer sans sélection | aucune | chaque appariement a son brouillon : tous deviennent démarrables sans sélection. Démarrer verrouille l'équipe (`MatchReporting`) et passe le match « en cours » : ce doit rester un geste explicite, jamais l'ouverture du lien. **Perte acceptée** (2026-11-01) : on ne change plus les équipes ni la journée d'un match du calendrier par la sélection — le déplacement admin reste |
| Lecture seule | aucune | **acceptée** (2026-11-01) : dans les comptes rendus en cours à la livraison, l'autre coach que celui qui a confirmé est bloqué, même si c'est lui qui saisissait ; un compte rendu publié ne se corrige plus que par son démarreur ou un admin, quand l'un ou l'autre coach le pouvait |
| Journal des deux camps | **une colonne `recorded_by` et un remplissage** de `match_report_actions` depuis l'event store ; l'event store n'est pas touché | la carte vérifie sur un dump que chaque `ActionRecorded` porte son auteur |
| Match en direct | aucune | — |
| Étape MVP | aucune | les anciens MVP — à tout tour, zéro ou plusieurs par camp — restent valides : la règle « un par camp » ne vaut qu'à la saisie, sinon des comptes rendus publiés deviendraient illisibles ; l'étape sait afficher un camp qui en a déjà, en correction. SPP et événement publié inchangés. Les comptes rendus en cours ne sont pas bloqués |
| Tirage | un événement **ajouté** | ce qui sort du BC reste une action `Mvp` ; « tiré parmi » pour les seuls nouveaux |

## Ce que l'épic ne couvre pas

- **Le mode « Chacun son équipe »** : E22.
- **La mise à jour en direct** du journal chez l'autre coach, et les
  notifications : E23. Sans elles, la lecture seule se rafraîchit au
  rechargement.
- **Le panier et l'annulation des coups de pouce** : E20.
- **Changer de mode en cours de match** : exclu (décision du 2026-11-01).

## Terminé quand

Depuis le calendrier, un coach démarre un match en « Je saisis tout » sans
passer par la sélection, saisit les actions des deux camps depuis un journal
commun, enchaîne le MVP du domicile puis du visiteur — l'un tiré au sort parmi
six désignés —, et publie : sur ordinateur comme sur mobile. L'autre coach
ouvre le même match et le voit sans pouvoir y écrire. Un compte rendu commencé
avant la livraison se termine comme avant.
