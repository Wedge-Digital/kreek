# Départages — activation et calcul

Départage des équipes à égalité de points de classement. Deux volets : la **saisie**
de la configuration (quels critères sont actifs, dans quel ordre) dans le formulaire
de règles de la compétition, et le **calcul** du départage appliqué au classement.

Le départage est une fonction du BC `ranking` : le catalogue des critères et la
logique de comparaison lui appartiennent. Le BC `competitions` ne fait que **stocker
le choix du gestionnaire** dans ses règles de compétition et l'exposer via le port
ACL déjà en place (`competition_info_adapter`).

## État des lieux (constat initial)

- Le formulaire phase 2 de création de compétition (`new-competition-phase-2.html`)
  affiche **7 critères** ordonnables par drag & drop. Aucun n'est désactivable.
- Persistance : `RankingRules.additionnal_ranking_points: HashMap<String, u32>`
  (`competitions/domain/competition_rules.rs:52`) — une map `id → priorité`. Elle
  exprime l'ordre, **pas** l'activation, et est stringly-typée (`String`/`u32` nus,
  en infraction avec la règle « pas de primitives nues dans les agrégats »).
- **Aucun défaut n'existe** : `create_draft_competition` n'écrit pas de règles et
  `find_rules` renvoie `None` jusqu'à la première soumission de la phase 2. Le handler
  GET produit alors `existing_rules_json = "null"` et c'est le JS qui amorce la liste
  complète. (Les `HashMap::new()` de `save_competition_rules.rs:171` et
  `rules_labels.rs:64` sont des helpers de test, pas des défauts de production.)
- Le calcul du départage **n'existe pas** : explicitement hors scope de la feature 1
  du BC `ranking` (cf. `../README.md`, « Périmètre feature 1 »).
- Aucun garde-fou de démarrage n'existe : `save_competition_rules::execute` ne
  vérifie que `RosterInMultipleTiers`.

## Catalogue des critères (7)

| Critère | Libellé formulaire | Compteur | Sens |
|---|---|---|---|
| `diff_td` | Différence de touchdowns (marqués − encaissés) | dérivé (`nb_td − nb_td_conceded`) | décroissant |
| `nb_td` | Nombre de touchdowns marqués | cumul des `own_td` | décroissant |
| `nb_td_conceded` | Nombre de touchdowns encaissés | cumul des `opponent_td` | **croissant** — le moins est le mieux |
| `nb_cas` | Nombre de blessures infligées | cumul des actions `Sortie` **uniquement** | décroissant |
| `nb_wins` | Nombre de victoires | **existe déjà** (`WinCount`) | décroissant |
| `nb_fouls` | Nombre de fautes commises | cumul des actions `Agression` | décroissant — le plus est le mieux |
| `nb_reu` | Nombre de réussites | cumul des actions `Passe` **uniquement** (ni `Interception`, ni `Lancer`) | décroissant |

`nb_cas` compte les `Sortie` **strictement**, pas les `Blesse` — même définition que le
bonus agressif livré par la feature `ranking-bonus-points`. Une seule sémantique de
« blessure infligée » dans toute l'application.

`nb_fouls` est comparé en **décroissant** : une équipe qui agresse davantage est
avantagée au départage. Choix de ligue assumé.

`nb_td_conceded` est **nouveau** (ajouté en phase 1).

`nb_red_cards` (« Nombre de cartons rouges ») a été **retiré du catalogue en phase 3** :
`MatchActionType` (`match_report/domain/value_objects.rs:151`) n'expose ni carton
rouge ni expulsion — le critère vaudrait 0 pour toutes les équipes. À réintroduire le
jour où les expulsions seront saisies dans le rapport de match.

Sémantique des compteurs et sens de comparaison **validés en phase 3** (colonnes
ci-dessus).

## Mécanisme de départage (unité `tiebreak-calc`)

Chaque critère du catalogue a un **compteur cumulé par équipe**, mis à jour à chaque
match publié et porté par la ranking line (`CumulativeTotals`). Au classement, les
lignes sont d'abord ordonnées par `ranking_points` ; **à égalité de points**, les
compteurs des critères actifs sont comparés dans l'ordre de priorité configuré,
jusqu'à ce que l'un départage. Si tous donnent l'égalité, les équipes restent ex æquo
(règle 5).

**Décision (phase 3)** : les compteurs sont accumulés **pour tous les critères,
toujours**, indépendamment de la configuration. L'activation ne joue qu'au moment
d'ordonner. Le calcul est ainsi découplé de la configuration, la projection reste
rejouable sans connaître l'historique des règles, et un changement de configuration
ne produirait pas de compteur définitivement faux.

État de l'existant : `CumulativeTotals` (`ranking/domain/ranking_line.rs:59`) accumule
`matches_played`, `wins`, `draws`, `losses`, `ranking_points`. Les TD et blessures
existent par match dans `MatchStats` (introduit par la feature bonus) mais ne sont pas
cumulés. Il reste donc **5 compteurs à ajouter** (`nb_td`, `nb_td_conceded`, `nb_cas`,
`nb_fouls`, `nb_reu`) et **un dérivé** (`diff_td`).

## Règles métier validées (phase 1)

1. **Au moins un critère coché** — une configuration où tous les critères sont
   décochés est refusée.
2. **L'ordre d'un critère décoché est conservé** — le recocher le remet à sa place.
   Conséquence : un critère inactif reste présent dans la configuration persistée,
   avec sa priorité, plus un flag d'activation.
3. **Défaut à la création d'une compétition** : les 7 critères actifs, dans l'ordre
   du catalogue ci-dessus. À matérialiser côté domaine, pas côté front.
4. **Activation et ordre sont figés après le démarrage de la compétition.**
5. **Les ex æquo résiduels sont assumés** — si tous les critères actifs donnent
   l'égalité, les équipes restent ex æquo au classement. Pas de départage ultime
   (pas de tirage au sort).

## Points ouverts

- ~~**Périmètre du gel (règle 4)**~~ — **tranché en phase 2** : aucun travail requis.
  Le formulaire de règles n'existe que dans le parcours de création, il n'y a pas de
  route d'édition après création. La règle est déjà vraie de fait ; elle devient une
  contrainte pour la future page d'admin des règles. Pas de carte séparée.
Aucun point ouvert : la sémantique des compteurs et le sens de comparaison ont été
tranchés en phase 3.

Le projet n'étant pas en production, aucune reprise de données n'est à prévoir : les
configurations existantes sont des brouillons sans valeur à préserver.

## Découpage en unités

| Unité | Portée | UI |
|---|---|---|
| `competition-rules-form` | Cases à cocher dans le formulaire phase 2 + évolution du modèle persisté (ordre + activation) + règles 1 à 4 | Oui |
| `tiebreak-calc` | Application de l'ordre de départage au classement dans le BC `ranking` (propagation ACL + comparateurs) | Non |

Ordre de traitement : **`competition-rules-form` d'abord**, puis `tiebreak-calc`.

## Progression

| Unité | Mockup | Front | Back | DTOs | Use cases | Domaine | Intégration | Cartes |
|---|---|---|---|---|---|---|---|---|
| competition-rules-form | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ |
| tiebreak-calc | n/a | n/a | | | | | | |
