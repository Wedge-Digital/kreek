# E25 — Retrouver ses matchs

**État :** `to_be_refined` — 4 cartes à écrire. Découpage validé le 2026-11-01. Née le 2026-11-01 des maquettes
mobiles. Dépend de E21 pour « Commencer » ou « Continuer ».

**Maquette :** `app-mobile-matches.html`. Pages neuves : le workflow feature s'y
applique. **Se livre avant E24**, dont l'onglet « Matchs » mène ici.

## La fonction

Un coach a du mal à trouver ses matchs. Aucune page ne les rassemble : on ne
les voit qu'équipe par équipe (onglet Matchs de la fiche, E06) ou compétition
par compétition (Calendrier, Résultats).

« Mes matchs » est une page **sans équivalent sur ordinateur**, que seule la
barre d'onglets mobile référence (décision du 2026-11-01) ; c'est une route
ordinaire, simplement absente de la barre latérale. Elle liste les matchs **à jouer** de toutes les équipes du coach **inscrites
dans une saison en cours** (décision du 2026-11-01). **Seul le prochain match de chaque équipe** porte un bouton :
« Commencer » si rien n'est saisi, « Continuer » si la saisie a commencé. Le
prochain match d'une équipe est sa première rencontre programmée non jouée ; il
se saisit quelle que soit sa date. « Nouveau match » démarre un match hors
calendrier : c'est sur mobile la seule entrée hors calendrier, l'onglet « Match »
disparaissant (E24). Elle reprend sa règle — masquée quand une compétition de
l'espace interdit le hors-calendrier.

La donnée existe : `list_team_matches` (`competitions`) trie déjà « en cours »
puis « à venir » — son commentaire parle de « mon prochain match » —, mais
aucune requête ne renvoie le seul prochain match, et `in_progress` ne dit pas
dans quel mode la saisie a commencé.

## Les cartes

| # | Intitulé | Apport |
|---|---|---|
| — | Le prochain match d'une équipe | une requête et un widget dans `competitions` |
| — | L'état de saisie d'un match | pas commencé, ou commencé et par qui : servi par `match_report`, qui décide « Commencer » ou « Continuer » |
| — | La page « Mes matchs » | les matchs à jouer des équipes du coach inscrites dans une saison en cours, résolues depuis la session et jamais depuis l'URL ; bouton sur le seul prochain match ; « Nouveau match », masqué si le hors-calendrier est interdit |
| — | Les tests e2e de « Mes matchs » | dont le match lointain qui reste le prochain ; reprise du volet mobile de `test_competition_hors_calendrier` |

## Ce qui commande l'ordre

Le prochain match et l'état de saisie sont indépendants ; la page les compose
(souveraineté des données : chaque BC sert son widget, la page hôte n'a pas de
logique).

## Conséquences sur les données de production

**Lecture seule** : rien n'est écrit ni migré. Ce qui compte est que l'existant
s'affiche juste :

- le prochain match se lit dans `competition_match_display_proj`, déjà triée
  pour cela par `list_team_matches` ; un match hors calendrier en cours y figure
  (carte 427, faite) ;
- une saison par équipe, résolue par son inscription — la règle actuelle,
  reprise telle quelle ;
- chaque appariement ayant son brouillon, tous les matchs à venir affichent
  « Commencer » ; les comptes rendus en cours à la livraison affichent
  « Continuer », en « Je saisis tout » (règle de E21).

## Ce que l'épic ne couvre pas

- **L'onglet « Joués »** : pas nécessaire à ce stade, prévu plus tard (décision
  du 2026-11-01).

- **Le cartouche d'appel à l'action de l'accueil** (prochain match ou
  évolutions à faire, maquettes `app-mobile-home*.html`) : reporté à plus tard
  (décision du 2026-11-01) — on fluidifie d'abord la saisie. Il réutilisera le
  prochain match et l'état de saisie posés ici ; reste à décider où vit sa règle
  « évolutions avant prochain match », qui croise `teams` et `competitions`.
- **L'onglet Matchs de la fiche équipe** : E06.

## Terminé quand

Un coach à deux équipes ouvre « Mes matchs » et voit, pour chacune, son prochain
match avec « Commencer » — ou « Continuer » si la saisie a commencé —, même
programmé des semaines à l'avance ; les autres matchs à jouer n'ont pas de
bouton.
