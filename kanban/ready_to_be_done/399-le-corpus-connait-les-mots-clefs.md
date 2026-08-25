# Le corpus connaît les mots-clefs

**Priorité : haute**
**Dépend de :** rien — première carte de la Haine
**Conception :** `docs/specs/haine/saisie-des-actions/03-back.md`
**Fichiers :** `assets/references.example/keywords_fr.json`,
`assets/references.example/skills_fr.json`,
`assets/references.example/teams_fr.json`,
`src/app/references/domain/models.rs`, `domain/port.rs`,
`io/repository/in_memory_reference_repository.rs`

## Objectif

Le référentiel sait répondre à deux questions qu'il ignore aujourd'hui : quels
mots-clefs le règlement connaît-il, et lesquels une ligne de roster porte-t-elle.

```rust
fn list_keywords(&self) -> &[Keyword];
fn find_keyword_by_uid(&self, uid: &str) -> Option<&Keyword>;
```

## Le fichier

`keywords_fr.json`, même forme que `special_rules_fr.json` :

```json
{
  "bloodbowl_version": "2025",
  "edition": "Third Season Edition",
  "keywords": [ { "uid": "DARK_ELF", "label": "Elfe Noir" }, … ]
}
```

Trente-huit entrées. Le corpus de production les fournira, ainsi que des
fichiers `*_en.json` — **qui ne changent rien ici** : le chargement lit les noms
en dur, comme les onze autres fichiers, et le choix d'une langue appartient à la
carte 395. Ne pas l'anticiper sous prétexte que les fichiers sont là.

## Les lignes de roster portent leurs mots-clefs

```json
{ "uid": "KHORNE__BLOODSEEKER", "positionName": "Rabatteur Sanglant",
  "keywords": ["BLOCKER", "HUMAN"], "cost": 105, … }
```

```rust
pub struct PlayerPosition {
    // …
    #[serde(default)]
    pub keywords: Vec<String>,
}
```

**`#[serde(default)]` n'est pas optionnel.** Sans lui, tout corpus dépourvu du
champ cesserait de charger — et `load_references` ne se contente pas d'échouer :
il **empêche le démarrage**. Un champ absent doit donner une liste vide, pas un
serveur mort.

## Les trente-huit compétences

Le jeu de démonstration gagne `HAINE_<UID>` pour chaque mot-clef, en catégorie
**`TRAITS`** :

```json
{ "uid": "HAINE_DARK_ELF", "name": "Haine : Elfe Noir", "category": "TRAITS",
  "type": "Standard", "activation": "Passive",
  "description": "Ce joueur hait les Elfes Noirs." }
```

`TRAITS` **n'est dans l'accès d'aucun poste** — vérifié sur tout le corpus. Deux
conséquences obtenues sans écrire une ligne : les Haines n'apparaîtront pas dans
le sélecteur de compétences, et `resolve_skill_cost` les refusera par
`CategoryNotAccessible`. La mise à plat tient à ce fait ; **le vérifier fait
partie de la carte**.

`type: "Standard"` parce que le champ ne connaît que « Standard » et « Élite ».
Une Haine n'est ni l'un ni l'autre, mais elle n'est jamais achetée : la valeur
est sans conséquence. C'est écrit ici pour que personne ne s'interroge plus tard.

## La garde

Au démarrage, si le corpus ne porte **aucun** mot-clef, la fonctionnalité serait
muette : un sélecteur vide, sans que rien ne le dise. Le chargement doit échouer
bruyamment, comme le prévoit la carte 388 pour `LOW_COST_LINEMEN`.

À trancher à l'implémentation : refus de démarrer, ou `error!` au journal. Le
premier est cohérent avec le reste de `load_references` ; le second évite qu'un
corpus incomplet bloque une plateforme dont le reste marche.

## Checklist

- [ ] `keywords_fr.json` dans le jeu de démonstration, 38 entrées
- [ ] `Keyword { uid, label }` dans `models.rs`, chargé au démarrage
- [ ] `PlayerPosition.keywords` en `#[serde(default)]`
- [ ] `keywords` posés sur les lignes de roster du jeu de démonstration
- [ ] 38 compétences `HAINE_<UID>` en catégorie `TRAITS` dans `skills_fr.json`
- [ ] `list_keywords` et `find_keyword_by_uid` au port
- [ ] Garde sur corpus sans mot-clef
- [ ] Tests unitaires :
  - [ ] les 38 mots-clefs sont chargés, un uid connu est trouvé, un inconnu rend `None`
  - [ ] une ligne de roster sans `keywords` charge et rend une liste vide
  - [ ] **`TRAITS` n'est dans l'accès d'aucun poste du corpus** — c'est ce test
        qui protège la mise à plat
  - [ ] `resolve_skill_cost` sur une `HAINE_*` rend `CategoryNotAccessible`
- [ ] `make lint`, `make check-arch`, `make test`
