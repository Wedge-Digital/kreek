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
- Le défaut à la création est une map **vide** (`save_competition_rules.rs:171`,
  `rules_labels.rs:64`) ; c'est le JS qui retombe sur la liste complète à l'affichage.
- Le calcul du départage **n'existe pas** : explicitement hors scope de la feature 1
  du BC `ranking` (cf. `../README.md`, « Périmètre feature 1 »).
- Aucun garde-fou de démarrage n'existe : `save_competition_rules::execute` ne
  vérifie que `RosterInMultipleTiers`.

## Catalogue des critères (8)

| Critère | Libellé formulaire |
|---|---|
| `diff_td` | Différence de touchdowns (marqués − encaissés) |
| `nb_td` | Nombre de touchdowns marqués |
| `nb_td_conceded` | Nombre de touchdowns encaissés |
| `nb_cas` | Nombre de blessures infligées |
| `nb_wins` | Nombre de victoires |
| `nb_fouls` | Nombre de fautes commises |
| `nb_reu` | Nombre de réussites |
| `nb_red_cards` | Nombre de cartons rouges |

`nb_td_conceded` est **nouveau** (ajouté en phase 1) ; les 7 autres existent déjà
côté formulaire. Le sens de comparaison de chaque critère (croissant / décroissant)
reste à préciser — traité avec l'unité `tiebreak-calc`.

## Règles métier validées (phase 1)

1. **Au moins un critère coché** — une configuration où tous les critères sont
   décochés est refusée.
2. **L'ordre d'un critère décoché est conservé** — le recocher le remet à sa place.
   Conséquence : un critère inactif reste présent dans la configuration persistée,
   avec sa priorité, plus un flag d'activation.
3. **Défaut à la création d'une compétition** : les 8 critères actifs, dans l'ordre
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
- **Sens et sémantique de chaque critère** : à détailler avec l'unité `tiebreak-calc`.

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
| competition-rules-form | ✅ | ✅ | | | | | | |
| tiebreak-calc | n/a | n/a | | | | | | |
