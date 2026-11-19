# BC `players` — L'agrégat panier de customisation

**Priorité : haute**
**Dépend de :** `302-players-customisation-domain.md`
**Contexte :** `players` — domaine

## Objectif

L'agrégat qui porte les lignes en attente et **tous** les invariants de
validité. Pur et synchrone : aucun `async`, aucun port.

**Spec :** `docs/specs/player-customisation/player-detail/06-domaine.md`.

---

## Conception

Calqué sur `teams::domain::recruitment_basket`, dont il partage la nature : des
invariants forts, une hydratation qui lui apporte ce dont ses gardes ont
besoin, puis du pur synchrone.

```rust
pub enum CustomisationLine {
    Skill { id: BasketLineId, skill_id: SkillId },
    Stat  { id: BasketLineId, stat: StatKind, crans: StatCrans },
    Price { id: BasketLineId, delta: KpoDelta },
    Spp   { id: BasketLineId, amount: SppAmount },
}

pub struct CustomisationBasket {
    player_id: PlayerId,
    version:   BasketVersion,
    lines:     Vec<CustomisationLine>,
    // hydratés, jamais persistés
    base_stats:     ResolvedStats,
    owned_skills:   Vec<SkillId>,
    catalog_skills: Vec<SkillId>,
    current_value:  Kpo,
    current_spp:    Spp,
}
```

`base_stats` porte les caractéristiques **déjà résolues** — l'agrégat n'a pas à
connaître le catalogue de postes, il reçoit un point de départ et raisonne en
deltas.

## Les gardes, une par règle de la phase 1

| Garde | Refus |
|---|---|
| `add_skill` | inconnue du catalogue → `UnknownSkill` ; déjà possédée (base, SPP, ou en attente) → `SkillAlreadyAcquired` |
| `add_stat` | résultat hors bornes → `StatOutOfBounds` |
| `adjust_price` | résultat < 0 → `NegativePlayerValue` |
| `add_spp` | borné par le VO, la garde ne rejoue pas la contrainte |
| `remove_line` | ligne absente → `BasketLineNotFound` |

## `validate_all`

Rejoue les lignes sur un panier cloné vidé — chacune est donc jugée contre
l'état accumulé des précédentes. Deux améliorations d'agilité depuis `2+` ne
peuvent pas passer si la seconde franchit `1+`.

**Tout ou rien** : `Result<Vec<AppliedCustomisation>, Vec<RejectedLine>>`.

## `action_for_*`

Rend l'`ActionState` du projet — `Allowed`, `Blocked { cause }`,
`Forbidden { cause }` — qui alimente le grisage des boutons.

`Forbidden` est structurel et ne changera pas en vidant le panier (compétence
possédée de base). `Blocked` est conjoncturel — une borne atteinte à cause des
lignes en attente, qui se libère si on en retire une.

---

## Checklist

- [x] `CustomisationLine` et `CustomisationBasket`
- [x] `hydrate()`
- [x] Les cinq mutations avec leurs gardes
- [x] `validate_all()` — rejeu sur panier vidé
- [x] `action_for_stat()` / `action_for_skill()`
- [x] `effective_stat()` / `effective_value()` / `effective_spp()`
- [x] Test : deux améliorations d'AG depuis `2+` — la seconde est refusée
- [x] Test : une compétence déjà au panier est refusée en doublon
- [x] Test : une compétence possédée **de base** est `Forbidden`, pas `Blocked`
- [x] Test : une borne atteinte par le panier est `Blocked`, et se libère au
      retrait de la ligne
- [x] Test : prix cumulé sous zéro refusé
- [x] Test : `validate_all` rejette tout si une seule ligne tombe

---

## Notes d'implémentation

**`validate_all` réutilise les mutations elles-mêmes** — cloner, vider,
ré-ajouter ligne par ligne. Les gardes du rejeu sont donc exactement celles de
l'ajout : une seconde implémentation des règles pour la validation aurait fini
par diverger de la première.

**Les identifiants de ligne sont protégés contre la réattribution.** Un simple
`len() + 1` redonnerait l'identifiant d'une ligne retirée, et un retrait
supprimerait alors deux lignes. La boucle qui cherche un identifiant libre
existe pour ça, et un test la garde.

**`add_spp` n'a aucune garde**, délibérément : le plafond de 100 est **par
opération**, donc entièrement porté par `SppAmount`. Le total du joueur n'est
pas borné — un test le dit explicitement, sans quoi quelqu'un ajouterait un
plafond de total en croyant corriger un oubli.

**`ActionState` est un type propre à `players`**, alors que `teams` en a un
homonyme : les BCs ne partagent pas leurs types. Duplication assumée, imposée
par la souveraineté.
