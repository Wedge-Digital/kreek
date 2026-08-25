# Les compétences Élite valent 10 kPo de plus

**Priorité : haute**
**Dépend de :** 386
**Fichiers :** `assets/references.example/improvement_values.json`,
`src/app/references/io/repository/in_memory_reference_repository.rs`,
`src/app/references/domain/port.rs`,
`src/infrastructure/players/skill_catalog_adapter.rs`,
`src/app/players/ports.rs`,
`src/app/players/use_cases/improvement_cost_service.rs`,
`src/app/players/io/app_events/team_created_listener.rs`,
`src/app/players/domain/events.rs`,
`src/app/players/domain/player.rs`,
`src/app/players/io/repository/player_repository.rs`,
`src/infrastructure/data_migrations/`

## Objectif

Une compétence Élite ajoute **10 kPo de plus** à la valeur du joueur qu'une
compétence Standard de même accès. Le barème devient :

| | Standard | Élite |
|---|---|---|
| accès primaire | 20 | 30 |
| accès secondaire | 40 | 50 |

Les caractéristiques ne sont pas concernées : le bonus porte sur les
compétences.

## Les deux origines, pas une

Le bonus vaut pour l'achat en SPP **et** pour une compétence accordée à la
création d'une équipe expérimentée. La parité posée par la carte 249 — « une
compétence vaut le même prix quelle que soit son origine » — est donc
conservée ; c'est le barème commun qui change, pas la règle.

Concrètement, `initial_skill_value_delta` cesse d'ignorer `is_elite`. Le
drapeau est déjà calculé sur place dans `team_created_listener` (il part dans
l'événement `InitialSkillEarned`), et le commentaire qui dit qu'il « n'entre pas
dans ce calcul » disparaît avec.

Le mode d'acquisition ne change rien non plus : une Élite obtenue au hasard
vaut le même prix qu'une Élite choisie, comme le coût en SPP l'est déjà par
`random_for(is_elite)`.

## Le barème complet vit dans le corpus

`improvement_values.json` porte les quatre valeurs, sans rien à additionner.
Le fichier entier, dans sa forme cible :

```json
{
  "bloodbowl_version": "2025",
  "edition": "Third Season Edition",
  "improvement_values": {
    "skill": {
      "primary": 20,
      "primary_elite": 30,
      "secondary": 40,
      "secondary_elite": 50
    },
    "stat": { "ma": 20, "st": 60, "ag": 30, "pa": 20, "av": 10 }
  }
}
```

Deux clés ajoutées, aucune renommée, aucune supprimée : `primary` et
`secondary` gardent leur sens — le barème Standard — et le bloc `stat` ne bouge
pas, les caractéristiques n'ayant pas d'Élite.

La struct qui le lit, dans `in_memory_reference_repository.rs` :

```rust
#[derive(Debug, Clone, Deserialize)]
pub struct SkillImprovementValues {
    pub primary: u32,
    pub primary_elite: u32,
    pub secondary: u32,
    pub secondary_elite: u32,
}
```

Et la lecture devient un choix parmi quatre, au lieu d'un parmi deux :

```rust
fn improvement_skill_value_delta(&self, is_secondary_access: bool, is_elite: bool) -> u32 {
    let s = &self.improvement_values.skill;
    match (is_secondary_access, is_elite) {
        (false, false) => s.primary,
        (false, true)  => s.primary_elite,
        (true,  false) => s.secondary,
        (true,  true)  => s.secondary_elite,
    }
}
```

Pas de bonus à ajouter à une base : quatre cases, quatre nombres, et aucune
valeur de règle du jeu dans le code Rust. C'est la ligne de la carte 249, tenue
jusqu'au bout.

**Les deux champs sont obligatoires.** Un `serde(default)` ferait démarrer le
serveur sur un corpus incomplet, en appliquant silencieusement le barème
Standard aux compétences Élite : la règle serait inactive sans que rien ne le
dise. C'est exactement la forme d'échec que `CLAUDE.md` proscrit — « une étape
sautée doit échouer, pas rassurer ».

**Conséquence de déploiement** : le corpus de production vit hors du dépôt
(`REFERENCES__DIR`). Il doit porter `primary_elite` et `secondary_elite`
**avant** le premier démarrage du nouveau binaire, sinon `load_references`
refuse de démarrer. C'est voulu, et c'est bruyant.

## Le chemin du drapeau

`is_elite` existe déjà partout où il faut — `SkillCatalogEntryDto.is_elite`,
résolu par l'adapter depuis `skill_type == "Élite"`. Seules les signatures de
valeur l'ignorent :

- `IReferenceRepository::improvement_skill_value_delta(is_secondary, is_elite)`
- `ISkillCatalogPort::skill_value_delta(is_secondary, is_elite)`

Signature **modifiée**, et non seconde méthode : les deux appelants doivent
trancher explicitement, et une méthode « sans élite » laissée à côté serait le
piège où retomber.

## La migration

`players` est event-sourcé : **on ne réécrit pas l'histoire**. Les
`PlayerSkillPurchased` et `InitialSkillEarned` déjà écrits gardent le
`value_delta` calculé le jour où ils ont été émis.

Un nouvel événement domaine porte la correction :

```rust
PlayerValueRecalibrated {
    player_id: PlayerId,
    team_id:   TeamId,
    delta:     KpoDelta,
    reason:    RecalibrationReason,
}
```

`PlayerValueCustomised` n'est pas réutilisé : il dit « un commissaire a posé
cette valeur hors barème ». Le relire dans un an sur un joueur que personne n'a
touché induirait en erreur, et il déclenche en plus un recalcul de VEA dont on
n'a pas besoin ici — la carte 388 les recalcule toutes.

La migration, pour chaque joueur :

1. relit ses compétences acquises et résout leur élitisme dans le corpus,
2. appende **un seul** `PlayerValueRecalibrated` de `10 × n` s'il en a au moins
   une, rien sinon,
3. met la projection à jour dans la même transaction (`value_kpo + delta`).

Un joueur sans compétence Élite ne reçoit aucun événement : une migration ne
doit pas laisser une trace là où elle n'a rien changé.

## Checklist

- [x] `primary_elite` / `secondary_elite` dans le corpus d'exemple, champs
      **obligatoires** dans `SkillImprovementValues`
- [x] `improvement_skill_value_delta(is_secondary, is_elite)` — port et
      implémentation en mémoire
- [x] `skill_value_delta(is_secondary, is_elite)` — port `players`, adapter
- [x] `resolve_skill_cost` passe `skill.is_elite`
- [x] `initial_skill_value_delta(catalog, is_primary, is_elite)` + son appel
- [x] Événement `PlayerValueRecalibrated` : variante, `event_name()`, `apply`,
      branche de projection, et les deux `match` exhaustifs de l'historique
- [x] Migration enregistrée dans le registre de la carte 386
- [x] Tests unitaires : barème 20/30/40/50, parité création/achat étendue aux
      Élite, modes Chosen et Random équivalents, corpus incomplet refusé,
      migration (deux Élite, aucune Élite, rejeu)
- [x] Test e2e : achat d'une Élite, valeur du joueur vérifiée à l'écran
- [x] `make lint`, `make check-arch`, `make test` — 1249 tests

## Ce qui a été fait

Le barème vit **entièrement dans le corpus** : quatre cases, quatre nombres,
aucune valeur de règle du jeu dans le code Rust. Les deux signatures de port ont
été **modifiées** plutôt que doublées — une variante « sans élite » laissée à
côté aurait été le piège où retomber. Le compilateur a désigné les deux seuls
appelants, qui tranchent désormais explicitement.

`RecalibrationReason` est un enum et non un texte libre : ces événements se
relisent des années plus tard, et « pourquoi cette valeur a-t-elle bougé sans
que personne n'y touche » est la première question qu'on leur posera.

La migration écrit **dans la transaction du registre**, donc pas via
`IPlayerRepository::append` qui ouvre la sienne. Elle appelle les deux fonctions
transactionnelles que le dépôt expose déjà — le SQL d'écriture n'est pas
dupliqué, la migration écrit exactement comme l'application.

### Ce que le compilateur a trouvé

Deux `match` exhaustifs sur `PlayerDomainEvent` dans `match_history_service`,
que la carte ne listait pas. Ils rattachent chaque événement à un match ou
l'écartent explicitement : la nouvelle variante devait y être **nommée**, et
non absorbée par un `_ =>`. C'est ce qui garde ce service honnête quand un
événement apparaît.

### Le « vu échouer » n'a pas pu se faire à l'e2e

Ramener `primary_elite` à 20 ne fait pas échouer le test de navigateur : le
serveur de développement ne surveille pas les fichiers JSON, et le corpus reste
en mémoire tel qu'il était au démarrage. Le même sabotage fait en revanche
échouer le test unitaire du barème — `left: ValueKpo(20), right: ValueKpo(30)` —
qui lit le corpus directement. La chaîne est donc couverte des deux côtés, mais
la démonstration s'arrête à la frontière du processus.

### Le test e2e a d'abord échoué en suite, et passé seul

Il empruntait le joueur « riche » de la fixture, dont les tests précédents du
fichier ont déjà dépensé les SPP. Le bouton « Choisir » existe alors mais reste
**masqué** — `x-show` le remplace par « Budget insuf. » — et l'échec tombe sur
une attente de visibilité, loin de sa cause.

Le fichier suit une convention que je n'avais pas vue : **un joueur dédié par
test**, précisément pour que l'ordre d'exécution ne compte pas. La fixture en
crédite un cinquième, avec le commentaire qui dit pourquoi.

### Vérifié au vrai démarrage

La migration s'est exécutée sur la base de développement et s'est marquée dans
le registre. Zéro joueur corrigé, aucun n'y portant de compétence Élite : le
mécanisme est constaté sans effet de bord.

## Rappel de déploiement

Le corpus de production doit porter `primary_elite` et `secondary_elite`
**avant** le premier démarrage du nouveau binaire. Sinon `load_references`
refuse de démarrer — c'est voulu, et c'est bruyant.
