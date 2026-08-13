# Phase 3 — Architecture back — player-detail

## Widgets → BCs

Tout est dans `players`. Le mode customisation ne lit ni n'écrit la donnée d'un
autre BC : les compétences et les caractéristiques de base viennent de
`references` par le port existant, l'autorisation de `spaces` et `competitions`
par les ports existants. Aucun nouveau BC n'entre en jeu.

## Le panier est un agrégat, pas un objet applicatif

Reprise directe du patron de `teams` (`recruitment_basket.rs`), dont l'en-tête
énonce la règle :

> *« Il porte des invariants forts […]. La tension "le domaine n'appelle pas de
> port" se résout par **hydratation** : l'agrégat *porte* les données dont ses
> gardes ont besoin. Le use case hydrate, puis tout est pur et synchrone —
> aucun `async`, aucun port, aucune dépendance framework. »*

Les invariants de la phase 1 sont exactement de cette nature : bornes de
caractéristiques, doublon de compétence, prix plancher, plafond de SPP. Chacun
se juge contre l'état courant du joueur **et** les lignes déjà au panier — donc
contre un agrégat hydraté, pas contre une base interrogée au fil de l'eau.

**Seules les lignes sont persistées.** Le catalogue de compétences, les
caractéristiques de base et l'état du joueur sont rechargés à chaque
hydratation — c'est ce qui garantit qu'un panier vieux d'une heure est évalué
contre le joueur d'aujourd'hui.

## Fichiers

### Domaine

| Fichier | Nature | Contenu |
|---|---|---|
| `players/domain/customisation_basket.rs` | **nouveau** | L'agrégat panier : lignes, hydratation, gardes (bornes, doublon, plancher, plafond), `add_*` / `remove_line` |
| `players/domain/events.rs` | modifié | Quatre variantes de customisation — une par famille |
| `players/domain/player.rs` | modifié | `apply()` : quatre branches ; méthodes de commande produisant les événements |
| `players/domain/error.rs` | modifié | Variantes de refus : borne dépassée, compétence déjà présente, prix négatif, plafond SPP |
| `players/domain/value_objects.rs` | modifié | Bornes des caractéristiques, identifiant de ligne de panier |

Les quatre variantes rejoignent `PlayerDomainEvent` : la phase 1 pose que les
événements **existants** ne sont pas touchés, ce qui n'interdit pas d'en
ajouter — c'est déjà ainsi qu'ont été introduits `PlayerRenamed` et consorts.

### Application

| Fichier | Nature | Contenu |
|---|---|---|
| `players/use_cases/customisation_basket_hydration_service.rs` | **nouveau** | Charge joueur + catalogue + lignes, rend l'agrégat hydraté |
| `players/use_cases/customisation_basket_mutation.rs` | **nouveau** | Les cinq mutations unitaires — même forme, ne décident de rien |
| `players/use_cases/validate_customisation_use_case.rs` | **nouveau** | Applique le panier : un événement par ligne, puis suppression du panier |
| `players/use_cases/commands.rs` | modifié | Les commandes correspondantes |

### IO

| Fichier | Nature | Contenu |
|---|---|---|
| `players/io/web/widgets/player_customisation_widget.rs` | **nouveau** | `GET` du panneau |
| `players/io/web/customisation_controller.rs` | **nouveau** | Les sept `POST` |
| `players/io/web/templates/widgets/player-customisation-widget.html` | **nouveau** | Le panneau, repris de la maquette |
| `players/io/repository/customisation_basket_repository.rs` | **nouveau** | Implémentation Pg du port panier |
| `players/io/repository/player_repository.rs` | modifié | Branches de projection des quatre événements |
| `players/io/repository/projection_repository.rs` | modifié | Lecture des deltas cumulés |
| `players/io/web/player_detail_controller.rs` | modifié | `can_customise` resserré ; choix de l'occupant du slot |
| `players/routes.rs`, `players/router.rs` | modifiés | Routes et wiring |

### Migrations

| Migration | Contenu |
|---|---|
| `players__customisation_baskets` | Table de travail : `player_id` (PK), `space_id`, `state` JSONB, `version`, horodatages |
| `players_proj` — deltas de caractéristiques | Cinq colonnes de cumul, une par caractéristique |

## Routes

```
GET  /app/{space_id}/players/{player_id}/widgets/customisation
POST /app/{space_id}/players/{player_id}/customisation/skills/add
POST /app/{space_id}/players/{player_id}/customisation/stats/add
POST /app/{space_id}/players/{player_id}/customisation/price/adjust
POST /app/{space_id}/players/{player_id}/customisation/spp/add
POST /app/{space_id}/players/{player_id}/customisation/lines/remove
POST /app/{space_id}/players/{player_id}/customisation/validate
POST /app/{space_id}/players/{player_id}/customisation/cancel
```

## Ports

**`ICustomisationBasketRepository`** *(nouveau)* — calqué sur
`IPhaseBasketRepository` de `teams` : `load` / `save(expected_version)` /
`delete`, avec garde de version optimiste. Clé `player_id` seul : le panier est
propre au joueur, pas à son auteur (phase 2).

**`ISkillCatalogPort`** *(à étendre)* — il sait résoudre **une** compétence
(`find_skill`) mais **pas les lister**. L'onglet compétences a besoin du
catalogue complet, non filtré par l'accès du poste. Une méthode de listing est
donc à ajouter.

C'est un élargissement assumé : jusqu'ici `players` ne consultait le catalogue
que pour des compétences qu'il connaissait déjà par leur identifiant.

**`IPlayerSpaceMemberPort`, `IPlayerCompetitionPort`** *(existants, inchangés)*
— l'autorisation resserrée se construit à partir des mêmes données ; c'est la
règle de composition qui change, pas les ports.

## Domain services

`customisation_basket_hydration_service` — seul point où les DTOs des ports
deviennent des objets domaine. Les handlers ne voient jamais un
`SkillCatalogEntryDto` : ils reçoivent l'agrégat hydraté.

## La refonte de la projection est plus large qu'elle n'en a l'air

La phase 1 décide que `players_proj` porte les **deltas cumulés** par
caractéristique. Aujourd'hui la projection n'en porte **aucun** : tout est
résolu à la lecture par `resolve_stats`, qui compose base + séquelles +
augmentations SPP.

Faire porter le cumul à la projection oblige donc à y écrire **toutes** les
sources, pas seulement la customisation :

- `PlayerStatIncreased` — augmentation achetée en SPP ;
- `InjurySustained { Sequel }` — malus de séquelle ;
- les quatre événements de customisation ;
- **`MatchImpactReverted`** — qui *défait* les contributions d'un match
  dépublié pour correction, séquelles comprises.

Ce dernier est le point délicat : il est **mince à dessein**, il énonce un fait
sans porter les montants à retrancher, ceux-ci vivant dans l'instantané
`last_match` de l'agrégat. Une projection qui cumule des deltas doit donc, sur
cet événement, savoir combien retirer — ce que seul l'agrégat sait.

**Décision : recalcul depuis l'agrégat, avec hydratation dans le chemin
d'écriture.** La projection ne s'incrémente pas — elle **repose** le cumul
résolu à chaque écriture.

L'alternative aurait été d'épaissir `MatchImpactReverted` pour qu'il porte les
deltas retranchés. Elle est écartée : elle gardait la projection en incrémental
pur mais revenait sur une décision documentée du BC, l'événement étant mince
*à dessein*.

Ce que le recalcul coûte : une hydratation dans le chemin d'écriture, donc une
projection qui n'est plus une simple traduction de l'événement reçu. Ce qu'il
apporte : l'insensibilité à l'ordre et aux rejeux, et surtout l'impossibilité
d'une dérive — un cumul recalculé ne peut pas s'écarter de l'agrégat, là où un
cumul incrémenté finit toujours par le faire.

## Un piège déjà payé dans `teams`, à ne pas repayer

`basket_mutation.rs` le documente :

> *« Aucune ne rend l'agrégat muté. Ce serait un cadeau empoisonné : `save` rend
> la nouvelle version sans la reposer sur l'agrégat […]. Un appelant qui le
> cuirait dans les `hx-vals` du prochain geste ferait échouer chaque second clic
> en écriture concurrente — le piège de la carte 264, qui ne se voit qu'en
> navigateur. Les handlers relisent, et c'est la seule façon correcte. »*

Les mutations de customisation suivent la même règle : elles ne rendent pas
l'agrégat, et le handler relit avant de rendre le panneau.

## Règles métier (identifiées phase 3)

- **L'autorisation resserrée est une composition, pas un nouveau port.**
  Commissaire de ligue et admin d'espace : les mêmes données que
  `check_admin_rights`, moins la branche « coach de l'équipe ».
- **Le panier est supprimé, jamais vidé**, à la validation comme à
  l'annulation — son existence commande l'affichage (phase 2).
- **Un refus de ligne ne touche pas le panier.** L'ajout est rejeté, les lignes
  déjà présentes restent.

## Points ouverts

- **Durée de vie d'un panier abandonné** (hérité de la phase 2).
- **Sort d'un panier visant un joueur renvoyé entre-temps** (hérité de la
  phase 2).
