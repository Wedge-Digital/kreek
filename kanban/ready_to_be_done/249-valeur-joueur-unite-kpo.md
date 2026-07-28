# Valeur du joueur — unité kPo unique et table de référence

**Priorité : haute**
**Dépend de :** rien — livrable seule
**Bloque :** 250, 251, 252
**Fichiers :** `src/app/references/domain/port.rs`,
`src/app/references/io/repository/in_memory_reference_repository.rs`,
`assets/references/improvement_values.json` (nouveau),
`src/infrastructure/players/skill_catalog_adapter.rs`,
`src/app/players/ports.rs`, `src/app/players/use_cases/improvement_cost_service.rs`,
`src/app/players/io/app_events/team_created_listener.rs`,
`src/app/shared_kernel/app_events/player_improvement_app_events.rs`,
`src/app/teams/io/app_events/player_improvement_listener.rs`

## Problème

### Deux unités dans la même colonne

Quatre chemins écrivent dans `players_proj.value_kpo`, tous via un `value_delta`
typé `ValueKpo` — et deux d'entre eux ne sont pas en kPo.

| Chemin | Source | Unité réelle |
|---|---|---|
| `PlayerCreated.starting_value` | `pos.cost` des données de référence (Lanceur = 75) | **kPo** |
| `InitialSkillEarned` | `skill_value_delta()`, codé en dur dans le listener | **kPo** |
| `PlayerSkillPurchased` | `improvement_skill_value_delta()` → 20 000 / 40 000 | **Po** |
| `PlayerStatIncreased` | `improvement_stat_value_delta_*()` → 10 000 à 60 000 | **Po** |

Le newtype s'appelle `ValueKpo` dans les quatre cas : il ment dans deux. C'est un
échec du principe des value objects — le type porte le nom de l'unité et ne
valide rien.

**Symptôme visible** : `player-table-fragment.html:73` affiche
`{{ p.value_kpo }} kPo`. Un Lanceur à 75 kPo qui achète une compétence s'affiche
à **20075 kPo**. La TV, elle, est juste — parce que
`player_improvement_listener.rs:66` divise par 1000.

### Deux tables de valeurs qui divergent

La valeur d'une compétence dépend de la façon dont elle a été obtenue :

- **À la création** — `team_created_listener.rs:80` :
  `base = primaire ? 20 : 40`, `+10 si la compétence est Élite`.
- **À l'achat par SPP** — `improvement_skill_value_delta(is_secondary)` :
  20 000 / 40 000, **`is_elite` n'est même pas transmis**.

La même compétence sur le même joueur ne vaut donc pas la même chose selon son
origine. Et la table de création est codée en dur dans un listener, alors que
**toutes** les autres tables de référence du projet vivent dans
`assets/references/`.

### Décisions prises

- Tout passe en **kPo** : l'île Po est la plus petite et la plus isolée, et le
  reste du projet (données de référence, `teams::Kpo`, nom de la colonne,
  affichage) est déjà en kPo.
- **Le bonus élite disparaît** : il n'a pas lieu d'être. `is_elite` n'aura plus
  aucun effet sur la valeur, dans aucun des deux chemins.
- Les bases dev et test sont **resettées** — aucune migration de payload, aucun
  upcasting.

## Action

### 1. Sortir la table de valeurs dans les données de référence

Nouveau fichier `assets/references/improvement_values.json`, sur le modèle des
autres fichiers du dossier :

```json
{
  "bloodbowl_version": "2025",
  "edition": "Third Season Edition",
  "improvement_values": {
    "skill": { "primary": 20, "secondary": 40 },
    "stat":  { "ma": 20, "st": 60, "ag": 30, "pa": 20, "av": 10 }
  }
}
```

Chargé par `InMemoryReferenceRepository` comme `skill_cost.json`
(`read_json::<ImprovementValuesFile>(dir, "improvement_values.json")`). Les six
fonctions `improvement_*_value_delta` de `references/domain/port.rs:39-44` lisent
désormais cette table au lieu de retourner des constantes.

### 2. Faire consommer cette table par le chemin de création

`skill_value_delta(is_primary, is_elite)`
(`team_created_listener.rs:80-84`) est **supprimée**. Le listener consulte le
catalogue comme le fait `improvement_cost_service`, et n'applique plus aucun
bonus élite.

`is_elite` reste un champ de `InitialSkillEarned` (information d'historique), il
n'a simplement plus d'effet sur la valeur.

### 3. Propager le kPo jusqu'à `teams`

`player_improvement_app_events.rs` : le champ `value_delta_po` devient
`value_delta_kpo`, et le commentaire de tête est corrigé.

`player_improvement_listener.rs:64-66` : la division par 1000 disparaît.

```rust
// avant
let value_delta = Kpo(value_delta_po / 1000);
// après
let value_delta = Kpo(value_delta_kpo);
```

**Point de vigilance** — si cette division est retirée sans que la table passe en
kPo (ou l'inverse), `team_value` devient silencieusement fausse d'un facteur
1000. Les deux changements doivent être dans le même commit.

*(Cette chaîne sera entièrement supprimée par la carte 251. Si les deux cartes
sont prises dans la même session, le renommage du champ est inutile — seule
compte la cohérence de l'unité au moment du commit.)*

### 4. Corriger les fakes de test

Trois fakes retournent des valeurs en Po ou en dur :
`increase_stat_use_case.rs:64` (`20_000`), `player_stats_service.rs:93-94`,
`post_login.rs:290`.

## Ce qui n'est pas dans cette carte

- **La distinction aléatoire / choisi.** `improvement_skill_value_delta` ne reçoit
  ni le mode ni le niveau : une compétence primaire tirée au hasard et une
  primaire choisie ajoutent la même valeur, alors que leur coût en SPP diffère
  (3 contre 6 au niveau 1). À vérifier contre les règles Third Season Edition —
  carte séparée si la règle l'exige.
- **`chosenElite` / `randomElite`.** `SkillCostLevel` prévoit ces champs mais
  `skill_cost.json` n'en contient aucun : `chosen_for(is_elite)` retombe toujours
  sur la valeur non-élite. Code sans données derrière, à nettoyer un jour — hors
  périmètre ici, il s'agit du coût en SPP, pas de la valeur.

## Checklist

- [ ] `improvement_values.json` créé et chargé par le repository de référence
- [ ] Les six `improvement_*_value_delta` retournent des kPo lus dans le fichier
- [ ] `skill_value_delta()` supprimée de `team_created_listener.rs`
- [ ] Le chemin de création consulte la même table que le chemin SPP
- [ ] Aucun bonus élite sur la valeur, dans aucun chemin
- [ ] `value_delta_po` → `value_delta_kpo`, division par 1000 retirée
- [ ] Fakes de test corrigés (3 fichiers)
- [ ] Test unitaire : la même compétence vaut le même prix à la création et à l'achat
- [ ] Bases dev et test resettées
- [ ] Vérifié en e2e : un joueur qui achète une compétence s'affiche à `base + 20` kPo, plus 20095
- [ ] `make check-arch` au vert, `make test` au vert
