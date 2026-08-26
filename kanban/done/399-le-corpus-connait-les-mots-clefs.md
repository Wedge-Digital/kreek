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
  "keywords": [
    { "uid": "BEASTMAN", "label": "Homme-Bête",
      "league_hate_selectable": true, "hate_skill_uid": "HAINE_BEASTMAN" },
    { "uid": "BLITZER", "label": "Blitzer",
      "league_hate_selectable": false },
    …
  ]
}
```

**Huit mots-clefs ne sont pas haïssables** : les six postes — Blitzer, Bloqueur,
Receveur, Coureur, Lanceur, Trois-quart — plus Gros Bras et Spécial. On hait une
espèce, pas un rôle. Le corpus le dit ; **aucune liste n'est écrite dans le
code**.

**`hate_skill_uid` est explicite, et non déduit.** La première conception
fabriquait la compétence par convention — `format!("HAINE_{uid}")`. Un corpus qui
aurait nommé sa compétence `HATE_BEASTMAN` aurait fait échouer la résolution en
silence, et la Haine ne serait jamais devenue une compétence. Le lien est
maintenant porté par la donnée, où il est vérifiable.

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
pub struct Keyword {
    pub uid: String,
    pub label: String,
    #[serde(default)]
    pub league_hate_selectable: bool,
    #[serde(default)]
    pub hate_skill_uid: Option<String>,   // présent si et seulement si sélectionnable
}

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

Le jeu de démonstration gagne une compétence **par mot-clef haïssable** — trente,
et non trente-huit : les huit exclus n'en ont pas, ce serait une compétence
inatteignable dans le corpus. En catégorie **`TRAITS`** :

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

## La garde — deux vérifications, pas une

`#[serde(default)]` sur `league_hate_selectable` vaut **`false`** : un corpus non
migré rendrait donc **zéro mot-clef haïssable**, et le sélecteur serait vide sans
que rien ne le dise. C'est précisément ce que la garde doit attraper.

Au démarrage, le chargement échoue si :

1. **aucun mot-clef n'est haïssable** — la fonctionnalité serait muette ;
2. **un mot-clef haïssable n'a pas de `hate_skill_uid`**, ou en désigne un qui
   **n'existe pas au catalogue de compétences** — la Haine serait déclarée puis
   perdue à la publication, sans un mot.

La seconde est ce qui remplace la convention de nommage : le lien n'est plus
supposé, il est vérifié au démarrage, une fois pour toutes.

## Checklist

- [ ] `keywords_fr.json` dans le jeu de démonstration, 38 entrées, dont 30
      haïssables portant leur `hate_skill_uid`
- [ ] `Keyword { uid, label, league_hate_selectable, hate_skill_uid }` dans
      `models.rs`, chargé au démarrage
- [ ] `PlayerPosition.keywords` en `#[serde(default)]`
- [ ] `keywords` posés sur les lignes de roster du jeu de démonstration
- [ ] **30** compétences de Haine en catégorie `TRAITS` dans `skills_fr.json` —
      une par mot-clef haïssable, aucune pour les huit autres
- [ ] `list_keywords` et `find_keyword_by_uid` au port
- [ ] Garde : aucun mot-clef haïssable → échec ; un haïssable sans
      `hate_skill_uid`, ou pointant une compétence absente → échec
- [ ] Tests unitaires :
  - [ ] les 38 mots-clefs sont chargés ; **30 sont haïssables**, les six postes
        et Gros Bras et Spécial ne le sont pas
  - [ ] un uid connu est trouvé, un inconnu rend `None`
  - [ ] chaque mot-clef haïssable désigne une compétence **qui existe**
  - [ ] une ligne de roster sans `keywords` charge et rend une liste vide
  - [ ] **`TRAITS` n'est dans l'accès d'aucun poste du corpus** — c'est ce test
        qui protège la mise à plat
  - [ ] `resolve_skill_cost` sur une `HAINE_*` rend `CategoryNotAccessible`
- [ ] `make lint`, `make check-arch`, `make test`
