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

`improvement_values.json` porte les quatre valeurs, sans rien à additionner :

```json
"skill": {
  "primary": 20,
  "primary_elite": 30,
  "secondary": 40,
  "secondary_elite": 50
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

- [ ] `primary_elite` / `secondary_elite` dans `improvement_values.json` de
      l'exemple, champs **obligatoires** dans `SkillImprovementValues`
- [ ] `improvement_skill_value_delta(is_secondary, is_elite)` — port `references`,
      implémentation en mémoire
- [ ] `skill_value_delta(is_secondary, is_elite)` — port `players`, adapter
- [ ] `resolve_skill_cost` passe `skill.is_elite` (déjà résolu pour le coût SPP)
- [ ] `initial_skill_value_delta(catalog, is_primary, is_elite)` + appel dans
      `team_created_listener`
- [ ] Événement `PlayerValueRecalibrated` : enum, `event_name()`, `apply` de
      l'agrégat, branche de projection
- [ ] Migration enregistrée dans le registre de la carte 386, **avant** celle de
      la carte 388
- [ ] Tests unitaires :
  - [ ] Élite primaire = 30, Élite secondaire = 50, Standard inchangée à 20/40
  - [ ] `une_competence_vaut_le_meme_prix_a_la_creation_et_a_l_achat_en_spp`
        étendu à une compétence Élite
  - [ ] mode Random et mode Chosen donnent la même valeur
  - [ ] corpus privé des champs élite → chargement refusé, message nommant le
        fichier fautif
  - [ ] migration : joueur à deux Élite → +20 et un seul événement ; joueur sans
        Élite → aucun événement ; rejeu → aucune écriture
- [ ] Test e2e : achat d'une compétence Élite en phase d'améliorations, valeur du
      joueur vérifiée à l'écran
- [ ] `make lint`, `make check-arch`, `make test`, tests e2e impactés
