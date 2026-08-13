# BC `players` — Domaine de la customisation

**Priorité : haute**
**Dépend de :** —
**Contexte :** `players` — domaine

## Objectif

Poser les fondations domaine du mode customisation : la table des directions
descendue depuis la couche applicative, les value objects, les quatre
événements et les méthodes qui les produisent. Aucun panier, aucun handler.

**Spec :** `docs/specs/player-customisation/player-detail/06-domaine.md`.

---

## La table des directions descend dans le domaine

C'est le cœur de la carte, et ce n'est pas un déplacement cosmétique.

`apply_increase()` vit aujourd'hui dans `use_cases/player_stats_service.rs` —
la couche **applicative** — parce que la résolution des caractéristiques a
besoin du catalogue, donc d'un port. Or l'agrégat panier (carte 304) doit
connaître cette table pour juger si une amélioration franchit une borne.

L'y laisser produirait **deux conventions**. C'est exactement le bug trouvé
dans la maquette en phase 1, où « Augmenter » dégradait le joueur sur trois
caractéristiques sur cinq.

```rust
// domain/match_impact.rs, auprès de StatKind
impl StatKind {
    /// Ce qu'un cran d'amélioration fait à la valeur brute.
    /// MV/FO/AR montent ; AG/PA sont des seuils de dé, ils descendent.
    pub fn improvement_step(self) -> i8 { … }
    /// Bornes inclusives de la valeur résolue.
    pub fn bounds(self) -> (u8, u8) { … }
}
```

Bornes : `Ma 0..9`, `St 0..9`, `Ag 1..6`, `Pa 1..6`, `Av 2..12`.

`player_stats_service::apply_increase` et `apply_malus` **consomment** ces
méthodes au lieu de porter la table. La séparation reste juste : le service
compose base et ajustements en interrogeant un port, le domaine dit dans quel
sens et entre quelles bornes.

## Value objects

| VO | Contrainte |
|---|---|
| `StatCrans(i8)` | non nul |
| `KpoDelta(i32)` | non nul, signé |
| `SppAmount(u8)` | `1..=100` |
| `BasketLineId(String)` | non vide |
| `CustomisationId(String)` | non vide |

## Événements

```rust
PlayerSkillCustomised { player_id, team_id, customisation_id, skill_id, author },
PlayerStatCustomised  { player_id, team_id, customisation_id, stat, offset, author },
PlayerValueCustomised { player_id, team_id, customisation_id, delta, author },
PlayerSppCustomised   { player_id, team_id, customisation_id, amount, author },
```

`offset` est **brut**, pas en crans : le domaine a traduit, et l'événement
enregistre ce qui a été réellement appliqué. Un rejeu ne doit dépendre d'aucune
convention externe.

**Ni `PlayerSkillCustomised` ni `PlayerStatCustomised` ne portent de
`value_delta`.** Il n'existe pas, il ne vaut pas zéro — seul le prix déplace la
valeur d'équipe (phase 1). Un champ à zéro inviterait quelqu'un à le remplir.

## Méthodes de commande

`customise_skill`, `customise_stat`, `customise_value`, `customise_spp` — elles
ne mutent pas `self`, elles rendent l'événement.

**Aucune garde de phase ni d'appartenance** : les customisations s'appliquent
toujours, un joueur renvoyé reste customisable. C'est ce qui les distingue de
`rename`, qui exige `membership == Active`.

Les invariants de validité sont joués par le panier (carte 304) : **le panier
est le gardien, `Player` est le greffier**.

## Nouvelles `DomainError`

`UnknownSkill`, `StatOutOfBounds { stat, bound }`, `NegativePlayerValue`,
`BasketLineNotFound`. `SkillAlreadyAcquired` est réutilisée — même fait métier,
quelle que soit l'origine.

---

## Checklist

- [ ] `StatKind::improvement_step()` et `StatKind::bounds()`
- [ ] `player_stats_service` consomme ces méthodes, ne porte plus la table
- [ ] Les cinq value objects
- [ ] Les quatre événements + `type_name()`
- [ ] `apply()` : quatre branches
- [ ] Les quatre méthodes `customise_*`
- [ ] Les quatre nouvelles `DomainError`
- [ ] Test : améliorer AG **descend** la valeur brute, améliorer AR la monte
- [ ] Test : `bounds()` couvre les cinq caractéristiques
- [ ] Test : un joueur `Dismissed` reste customisable
- [ ] Test : `apply()` de `PlayerSkillCustomised` ne touche pas `value`
- [ ] Test : `apply()` de `PlayerValueCustomised` déplace `value`
- [ ] Test de non-régression : `resolve_stats` rend les mêmes valeurs qu'avant
      le déplacement de la table
