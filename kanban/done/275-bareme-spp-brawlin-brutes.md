# Barème SPP — il dépend de la règle spéciale du roster

**Priorité : haute** — les SPP crédités aux joueurs sont faux pour six rosters
sur trente
**Dépend de :** rien
**Bloque :** 276
**Fichiers :** `src/app/references/domain/port.rs`,
`src/app/references/domain/models.rs`,
`src/app/references/io/repository/in_memory_reference_repository.rs`,
`src/app/players/ports.rs`,
`src/app/players/io/app_events/player_match_impact_listener.rs`,
`assets/references.example/` (corpus de démo, versionné),
`tests/e2e/test_special_rule_selector.py`

## Problème

Le barème d'acquisition des SPP est **codé en dur en Rust** :

```rust
fn touchdown_spp(&self) -> u8 { 3 }
fn casualty_spp(&self) -> u8 { 2 }
```

Or une équipe portant la règle spéciale **`BRAWLIN_BRUTES`** — « Brutes
Bagarreuses » — suit un barème inversé : le touchdown vaut **2** et la sortie
**3**. Six rosters sur trente la portent : `ORC`, `DWARF`, `BLACK_ORC`,
`KHORNE`, `NURGLE`, `OGRE`. Leurs joueurs gagnent aujourd'hui les mauvais SPP à
chaque match.

**`spp_rules.json` porte déjà les deux tables, et aucune ligne de Rust ne le
lit.** Le fichier attend depuis toujours :

| | TD | CAS | REU | MVP | INT | TTM |
|---|---|---|---|---|---|---|
| `normal` | 3 | 2 | 1 | 4 | 2 | 1 |
| `brawlin_brutes` | **2** | **3** | 1 | 4 | 2 | 1 |

Rien à reprendre sur l'existant : le projet n'est pas en production, et les SPP
déjà crédités dans l'event store restent tels quels.

## Action

### 1. Charger le fichier

`spp_rules.json` se lit comme `skill_cost.json` et `improvement_values.json`,
par `read_json::<T>(dir, …)` dans le dépôt de références. Rien à inventer, le
patron existe.

### 2. Le port doit connaître l'équipe

`touchdown_spp(&self) -> u8` ne prend **aucun argument** : il ne peut pas
dépendre d'un roster. Il faut lui passer le `roster_line_id` du joueur — la
seule information dont `players` dispose au moment du crédit — et laisser
`references` **résoudre lui-même** le roster puis le barème.

C'est `references` qui possède le corpus. Faire découper `DEMO_GRANIT__PIETAILLE`
sur `__` par `players` marcherait — la convention tient sur les trente rosters,
vérifié — mais ferait dépendre un BC d'une convention de nommage qui appartient à
un autre.

### 3. La sélection

Le roster porte `BRAWLIN_BRUTES` → table `brawlin_brutes`, sinon `normal`.

**L'association est mécanique** : le nom du barème est l'identifiant de la règle
en minuscules. C'est le résultat d'une correction du corpus — la clé s'écrivait
`brawling_brutes`, avec un G que l'identifiant de la règle n'a pas. Ne pas
réintroduire de table de correspondance entre deux orthographes : il n'y en a
plus qu'une.

Prévoir le cas du barème absent du fichier — un corpus tiers pourrait ne
déclarer que `normal`. Retomber sur `normal` plutôt que paniquer, et le dire.

### 4. Le port miroir et le listener

`players/ports.rs` duplique ces cinq méthodes, et
`player_match_impact_listener` les appelle. Les deux suivent la nouvelle
signature.

## Le corpus de démonstration — et le test qu'il casse

Aucun roster de démo ne porte la règle, et `spp_rules.json` de démo n'a pas de
table `brawlin_brutes` : **sans enrichissement, aucun e2e ne peut vérifier quoi
que ce soit.** C'est le réflexe que la série 255-271 a confirmé trois fois.

À faire, dans `assets/references.example/` — qui est **versionné**, contrairement
à `assets/references/` :

- ajouter la table `brawlin_brutes` à `spp_rules.json`, nettement distincte de
  `normal` pour qu'un test ne puisse pas passer par coïncidence ;
- déclarer `BRAWLIN_BRUTES` dans `special_rules_fr.json` ;
- la poser sur **`DEMO_GRANIT`**.

**Conséquence mesurée** : `test_special_rule_selector` fige les trois rosters de
démo — deux puces nommées pour les Granitiers, « Pas de règle spéciale » pour les
Zéphyriens, six options de select pour les Lanterniers. Passer les Granitiers à
trois règles casse une assertion, **à mettre à jour dans le même commit**.

Les Granitiers plutôt qu'un quatrième roster inventé : les fixtures e2e les
construisent déjà, et aucun test n'affirme de montant de SPP chiffré — vérifié
sur les neuf fichiers qui parlent de SPP. Les Zéphyriens sont à écarter : c'est
le seul roster sans règle, et le test en a besoin.

## Tests

**Unitaires** — un roster portant la règle, un sans, et un barème absent du
fichier. Le contraste compte : vérifier que `normal` reste `normal` pour les
autres, sinon un barème unique passerait pour une confirmation.

**E2E** — un match d'une équipe Granitiers : un touchdown crédite 2 SPP, une
sortie 3. À lire dans `players_proj`, pas seulement à l'écran.

## Checklist

- [ ] `spp_rules.json` réellement lu, plus aucune valeur de barème en dur
- [ ] Le port prend le `roster_line_id` ; `players` ne découpe aucune chaîne
- [ ] Barème absent du corpus → retour sur `normal`, tracé
- [ ] Corpus de démo enrichi : table, règle, et `DEMO_GRANIT` qui la porte
- [ ] `test_special_rule_selector` mis à jour dans le même commit
- [ ] Test unitaire du contraste entre les deux barèmes
- [ ] E2E : 2 SPP pour un TD, 3 pour une sortie, en base
- [ ] `make check-arch` au vert, `make test` au vert
