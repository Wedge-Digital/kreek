# E17 — Corriger le calendrier sans perdre la saisie

**État :** `ready_to_be_done` — 4 cartes prêtes (537 à 540), zéro faite.
Née de l'incident G. B. L. R du 2026-09-07, instruite sur la base de production
importée en local le 2026-09-08.
**Maquette :** `assets/rawpages/html/app-competition-admin-schedule-transfert.html`

## La fonction

Aujourd'hui, une rencontre programmée sur la mauvaise journée ne se corrige pas.
Elle se **détruit** : on annule le rapport, on le ressaisit ailleurs, ou on
supprime l'appariement. Cette épic ajoute le geste qui manque — *déplacer* — et
répare le lien cassé qui rend la suppression trompeuse.

## Ce qui l'a fait naître

Le 7 septembre, dans l'espace G. B. L. R, un coach a saisi un match sur la
journée 1 alors qu'il se jouait à la journée 15. Faute de pouvoir le déplacer,
il a annulé son rapport et tout ressaisi sur une rencontre créée à la main. Cinq
heures de saisie plus tard — vingt-cinq actions, deux publications, un
post-match — un administrateur a supprimé cet appariement pour retirer le
doublon. Le rapport a survécu à la suppression, invisible ; il a fallu un second
geste pour l'annuler.

**Résultat : les deux rapports sont annulés, et les données de la rencontre ne
sont plus atteignables** — elles existent encore dans l'event store, mais aucun
écran n'y mène.

Deux défauts distincts se sont additionnés, et l'épic les traite tous les deux.

## Les cartes

| N° | Carte | Vague |
|---|---|---|
| 537 | Un rapport de match peut changer de journée | 1 — le domaine |
| 538 | Le calendrier et les résultats suivent le déplacement | 2 — la propagation |
| 539 | Le tiroir de déplacement dans l'onglet Calendrier | 3 — l'écran |
| 540 | Un rapport manuel n'est jamais relié à son appariement | — indépendante |

## Ce qui commande l'ordre

**537 avant 538 avant 539** : l'événement, puis ce qu'il déplace, puis le geste
qui l'émet. Chaque carte compile et se teste seule ; seule la troisième se voit.

**540 est indépendante** et peut passer en premier. Elle ne concerne pas le
déplacement mais la suppression, et c'est elle qui a fait perdre les données de
l'incident : rien ne relie un rapport manuel à l'appariement fabriqué pour lui,
donc supprimer l'appariement laisse le rapport vivant — et la garde qui protège
un rapport publié ne s'applique pas non plus, faute de lien à suivre.

## Ce que la mesure a établi, et qui a réduit le chantier

L'intuition annonçait un gros œuvre. La lecture du code a dit l'inverse :

| Ce qu'on croyait | Ce qui est |
|---|---|
| `round_id` gouverne des règles | il a **deux** usages : une clé de dédoublonnage et des libellés |
| le classement dépend de l'ordre des journées | les écrans lisent l'état final ; les cumuls sont des sommes ; `ranking_lines.round_id` n'est **jamais relu** |
| il faudrait distinguer publié et non publié | aucun motif : le geste est le même dans les quatre états |
| la projection d'affichage se réécrit en SQL | trois listeners le font déjà sur app event ; il en faut un quatrième |

Le seul vrai coût est la recopie de `round_id` dans les quatre états de
l'agrégat, et les six colonnes de journée de `competition_match_display_proj` —
celles que lisent les onglets Résultats, Calendrier et Matchs d'une équipe.

## Ce que l'épic ne couvre pas

**La récupération des données de l'incident.** Les deux rapports du 7 septembre
sont annulés, et `Cancelled` est un état terminal : aucun chemin ne dé-annule.
Leurs quatre-vingts événements sont intacts dans l'event store, donc la
restauration est possible — par migration de données ou par ressaisie assistée.
C'est une décision d'exploitation, pas une fonctionnalité, et elle mérite sa
propre carte le jour où on la prend.

**La traçabilité des suppressions.** `PairingDeleted` ne porte aucun auteur : on
ne peut pas dire, depuis la base, qui a supprimé l'appariement du 7 septembre.
Sur une action destructrice réservée aux administrateurs, c'est un manque —
hors périmètre ici, à ouvrir à part.

## Terminé quand

Un administrateur qui trouve une rencontre sur la mauvaise journée la déplace en
deux clics, et la retrouve au bon endroit dans le Calendrier **et** dans les
Résultats — sans qu'aucune saisie ne soit perdue, et sans qu'il ait eu à
supprimer quoi que ce soit.
