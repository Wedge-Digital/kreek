# Lineman à vil prix

**Priorité : haute**
**Dépend de :** 386 et 387
**Fichiers :** `src/app/teams/domain/team_value.rs`,
`src/app/teams/use_cases/team_value_service.rs`, `src/app/teams/ports.rs`,
`src/infrastructure/teams/roster_catalog_adapter.rs`,
`assets/references.example/special_rules_fr.json`,
`assets/references.example/teams_fr.json`,
`src/infrastructure/data_migrations/`

## Objectif

La règle spéciale `LOW_COST_LINEMEN` — « Lineman a vil prix » — annule le prix
des linemen dans la **valeur d'équipe**. Les augmentations, elles, comptent
comme partout ailleurs.

Le lineman d'un roster est la ligne marquée `is_journeyman` dans le corpus :
c'est déjà cette ligne que `JourneymanTypeAdapter` désigne, et la règle n'en
introduit pas une seconde définition.

Sous la règle :

| | Compte pour |
|---|---|
| prix de base d'un lineman de l'effectif | 0 |
| compétences et caractéristiques de ce lineman | leur valeur pleine |
| journaliers virtuels (les places manquantes pour atteindre onze) | 0 |
| tout le reste — autres postes, relances, staff | inchangé |

Rien ne change pour la **trésorerie** : le coach paie toujours son lineman au
prix du corpus. C'est la VEA seule qui l'ignore.

## Ce que `teams` sait déjà

`SquadMemberDto` porte le `roster_line_id`, et `RosterCatalogDto.positions[]`
porte `cost` et `is_journeyman`. La soustraction se calcule donc entièrement
dans `teams` : **aucune modification du BC `players`**, aucune migration de
valeur joueur.

Il manque une seule chose au DTO : la règle. `RosterCatalogDto` gagne
`linemen_are_free: bool`.

## L'uid reste dans l'adapter

`LOW_COST_LINEMEN` est du vocabulaire de corpus. `teams` lit une règle —
« les linemen sont gratuits » — et pas un identifiant qu'il faudrait aller
comprendre ailleurs. C'est `roster_catalog_adapter.rs` qui traduit, à partir de
`team.special_rules` que `find_team_by_uid` rend déjà. Précédent :
`FAVOURED_OF_CHOOSE_`, en dur dans `team_creation`.

Une constante nommée, avec son commentaire, plutôt qu'un littéral au milieu
d'un `any()`.

## La règle vit dans le domaine

`compute_team_value` la porte, pas le service : « ce joueur compte-t-il pour
son prix ? » est une question de VEA, exactement comme « un indisponible vaut
zéro ».

```rust
pub struct ValuedPlayer {
    pub value_kpo: Kpo,
    pub available_for_next_match: bool,
    pub is_lineman: bool,
    pub base_cost: Kpo,
}

pub struct TeamValueInputs {
    // …
    pub free_linemen: bool,
}
```

- `players_value` : si `free_linemen && is_lineman`, le joueur compte pour
  `value_kpo.saturating_sub(base_cost)`.
- `journeymen_value` : zéro si `free_linemen`.

**La borne à zéro n'est pas décorative.** Un commissaire peut avoir baissé la
valeur d'un joueur par customisation (`PlayerValueCustomised` porte un delta
signé, et la projection l'écrête déjà à zéro). Un lineman dont la valeur est
passée sous son prix de base ne doit pas rendre la VEA négative : il compte
pour zéro, et c'est tout.

`build_inputs` remplit `base_cost` depuis le catalogue, en résolvant le poste
par `roster_line_id`. **Hypothèse assumée** : c'est le prix *actuel* du corpus,
pas celui payé le jour du recrutement. Changer le tarif d'un poste déplacera
les VEA — c'est déjà le comportement des relances et du staff, dont le
commentaire de `compute_team_value` dit qu'ils comptent « au prix de base, pas
au montant déboursé ».

## La migration

Rien à corriger joueur par joueur : la VEA est **une somme recalculée, jamais
une accumulation de deltas**. Il suffit de la recalculer.

La migration appelle `recompute_team_value_use_case::execute` sur **toutes**
les équipes — mécanisme nominal, qui appende un `TeamValueRecomputed` et met la
projection à jour. Toutes et pas seulement celles des rosters concernés :
placée après celle de la carte 387, elle rattrape du même coup les valeurs
joueurs corrigées par le bonus Élite.

Un `TeamValueRecomputed` est appendu **même si la valeur n'a pas bougé** —
c'est le contrat documenté du use case, la suite de ces événements étant
l'historique de progression de la TV.

### La garde : un corpus qui ne connaît pas la règle fait échouer la migration

La migration est one-shot, marquée en base une fois faite. Si le corpus de
production ne porte pas encore `LOW_COST_LINEMEN` au premier démarrage du
nouveau binaire, elle recalculerait toutes les VEA **sans** la règle, se
marquerait comme appliquée, et plus rien ne repasserait derrière : des valeurs
fausses, définitivement, et pas une ligne de journal pour le dire.

Elle vérifie donc, avant de commencer, qu'**au moins un roster du corpus porte
`LOW_COST_LINEMEN`**. Sinon elle échoue, et le démarrage est refusé — c'est la
même exigence que les champs obligatoires de la carte 387, et pour la même
raison : une règle inactive doit se voir.

Corollaire de déploiement : le corpus de production doit porter la règle sur
ses rosters concernés avant de déployer.

## Ce qui n'est pas retouché

Les VEA figées dans les rapports de match **publiés** (`home_team_value`,
`away_team_value`). C'est de l'historique : le match s'est joué sur ces
valeurs-là.

En revanche, un match **en cours de saisie** a déjà enregistré la VEA des deux
équipes en pré-match, et elle ne correspondra plus à la fiche équipe après
migration. Déployer quand aucun match n'est ouvert ; à défaut, l'écart est
connu et se résorbe au rapport suivant.

## Corpus d'exemple et e2e

`LOW_COST_LINEMEN` doit exister dans `special_rules_fr.json` et être posée sur
un roster de démonstration pour qu'un test e2e puisse l'exercer. **À vérifier
avant de choisir le roster** : plusieurs tests e2e affichent une valeur
d'équipe, et poser la règle sur un roster qu'ils utilisent déplacerait leurs
attentes.

## Checklist

- [x] `RosterCatalogDto.linemen_are_free`, rempli par l'adapter depuis une
      constante nommée
- [x] `ValuedPlayer.is_lineman` / `.base_cost`, `TeamValueInputs.free_linemen`
- [x] `players_value` avec `saturating_sub`, `journeymen_value` à zéro sous la règle
- [x] `build_inputs` résout le poste par `roster_line_id`
- [x] Migration de recalcul global, enregistrée après celle de la carte 387
- [x] Garde testée **dans ses deux branches**
- [x] `LOW_COST_LINEMEN` au corpus d'exemple, sur `DEMO_LANTERNE`
- [x] Six tests unitaires du domaine
- [x] Test e2e, vu échouer : `assert 440 < 440`
- [x] `make lint`, `make check-arch`, `make test` — 1258 tests

## Le roster retenu, et pourquoi

`DEMO_LANTERNE`. La carte demandait de vérifier l'impact e2e avant de choisir :
les trois rosters de démonstration sont exercés, mais celui-ci est **hors du
cycle `ROSTERS`**, et ses deux seuls tests lisent la trésorerie
(`test_recruitment_phase`) ou un message d'erreur
(`test_special_rule_selector`) — jamais une valeur d'équipe. La règle n'y
déplace aucune attente.

## Ce qui a été fait

Le domaine porte la règle, l'adapter traduit l'uid : `teams` lit « les linemen
sont gratuits », pas un identifiant de corpus. `load_players` résout le poste
par `roster_line_id` ; un poste introuvable donne « pas un lineman » et un prix
nul, de sorte que la règle ne s'applique jamais sur un prix inventé.

### La migration n'est pas atomique avec sa marque, et c'est assumé

Elle passe par `recompute_team_value_use_case`, qui ouvre sa propre transaction
par équipe — c'est ce qui en fait le mécanisme *nominal*. La règle d'atomicité
de la carte 386 y perd son effet.

Sans conséquence : l'événement porte une valeur **absolue**, pas un delta. Une
interruption laisse des équipes recalculées et d'autres non ; le rejeu repasse
sur toutes, et recalculer une valeur déjà juste la laisse juste. C'est
précisément ce qu'un delta ne permettrait pas. Le motif est écrit dans le
module plutôt que laissé à deviner.

### La garde n'était testée que du bon côté

Sa première version vivait dans la migration, et le seul test possible passait
par le corpus d'exemple — qui **porte** la règle. La branche d'échec, la seule
qui compte, n'était pas exerçable. Elle est extraite en fonction pure et
vérifiée dans les deux sens.

### Deux fausses manœuvres, à retenir

**Le premier test e2e passait avec la règle désactivée.** Il comparait l'équipe
à vil prix à une équipe d'un autre roster ; or les Lanterniers sont
naturellement moins chers que les Granitiers, et la comparaison mesurait cela.
Le discriminant retenu tient à l'arithmétique : hors règle, la valeur d'équipe
vaut la somme des joueurs **plus** relances et staff, donc elle ne peut pas lui
être inférieure. Un second test tient le sens inverse sur le témoin, sans quoi
l'assertion passerait aussi si toutes les valeurs étaient nulles.

**Et la « désactivation » qui a servi à le constater n'avait rien désactivé.**
`cargo fmt` avait replié l'expression sur une ligne, ma substitution
multi-lignes ne correspondait plus, et elle s'est appliquée en silence sur
zéro occurrence. Toute substitution de ce genre porte désormais son `assert`.

## Rappel de déploiement

Le corpus de production doit porter `LOW_COST_LINEMEN` sur ses rosters
concernés **avant** le déploiement, sinon la migration refuse le démarrage. Et
déployer quand aucun rapport de match n'est ouvert : un match en cours a
enregistré la valeur des deux équipes en pré-match, et l'écart ne se résorbera
qu'au rapport suivant.
