# Un journalier est un joueur

**Épic :** E15 — Recruter un journalier
**Ordre :** 1 · **Dépend de :** rien
**Conception :** `docs/specs/embaucher-un-journalier/` (`00-conception.md`,
`ecran-de-recrutement/03-back.md`)

## Objectif

Ouvrir un troisième statut d'appartenance, et faire en sorte que **cinq
lectures cessent d'exclure les journaliers** — trois en SQL, deux en Rust.
Aucun écran, aucun événement.

## Pourquoi elle est seule — et la plus risquée de la série

Elle change des lectures qu'**aucun compilateur ne vérifie**, sur les deux
sources d'effectif que l'application possède. Une erreur là-dessus se voit
partout et se diagnostique mal : un journalier invisible ne produit aucune
erreur, seulement un nombre faux au rapport suivant.

## Il y a deux lectures d'effectif, et c'est ce qui commande la carte

L'application lit l'effectif de deux façons, et **plusieurs écrans utilisent
les deux ensemble** :

| | Source | Filtre | Où |
|---|---|---|---|
| Projection | `players_proj` | SQL `membership = 'Active'` | `projection_repository.rs` |
| Event store | agrégats rejoués | Rust `is_active()` | `player_repository.rs:995` |

`build_player_rows` (`player_table_widget.rs:188`) appelle les deux pour un
seul tableau : la projection pour les lignes, l'event store pour les
caractéristiques et les SPP restants.

**Ouvrir l'un sans l'autre produit un joueur fantôme** : le journalier
apparaît dans le tableau, sans caractéristiques et à zéro SPP. Ni erreur, ni
trace — sur l'écran même que l'épic doit livrer. Les deux filtres ne sont pas
deux décisions, ce sont deux moitiés de la même.

## Conception

### 1. La troisième variante

```rust
pub enum RosterMembership { Active, Journeyman, Dismissed }
```

**Aucune migration** : la variante s'ajoute, les lignes existantes ne bougent
pas. La colonne est un `TEXT NOT NULL DEFAULT 'Active'` sans contrainte
`CHECK` (`migrations/20260730000001_players_proj_membership.sql`) — vérifié,
rien n'est dû côté base.

**Le piège est dans `from_str`, pas dans la colonne :**

```rust
pub fn from_str(valeur: &str) -> Self {
    match valeur {
        "Dismissed" => Self::Dismissed,
        _ => Self::Active,          // ← 'Journeyman' atterrit ici
    }
}
```

Le défaut permissif est délibéré — « tout ce qui n'est pas explicitement un
renvoi est une appartenance » — et il devient faux le jour où une troisième
valeur existe. **Sans bras explicite, un journalier redevient permanent au
premier rejeu de son agrégat**, en silence, et aucune des cinq lectures
ci-dessous ne le rattrape.

### 2. Les cinq lectures, et le contexte de chacune

C'est le cœur de la carte. Le filtre devient partout :

```sql
WHERE team_id = $1 AND membership <> 'Dismissed'
```

**Et non `IN ('Active','Journeyman')`** : la liste devrait être tenue à jour à
chaque nouvelle variante, alors que la question posée est bien « ce joueur
fait-il encore partie de l'effectif ? ».

#### Les trois requêtes SQL

| Fichier · ligne | Sert | Consommateurs | Pourquoi le journalier doit y figurer |
|---|---|---|---|
| `projection_repository.rs:36` (`lire_effectif`) | `find_by_team_id` et `find_alive_by_team_id` | `squad_adapter` → `teams` · `player_table_widget` · `match_player_selector_widget` · `player_data_adapter` → `match_report` | il compte dans la valeur d'équipe, il s'affiche, il joue |
| `projection_repository.rs:156` (`jerseys_by_team_id`) | les maillots pris | `player_creation::prochain_maillot_libre` | sinon **deux joueurs au même numéro** |
| `projection_repository.rs:174` (`count_available_by_team_id`) | le compte des alignables | `player_data_adapter` → `match_report` | sinon **on recrée des journaliers pour combler des journaliers** |

Le commentaire de `prochain_maillot_libre` raconte déjà cette histoire : la
carte 265 avait promis qu'un maillot renvoyé se libérait « d'office », et
c'était faux tant qu'une lecture échappait au filtre. « Le filtre
d'appartenance ne vaut que là où toutes les lectures passent. »

Le compte des alignables, lui, est celui qui décide combien de journaliers il
manque à l'étape 2 du rapport — celui dont la carte `495` vient de corriger
l'affichage.

#### Les deux prédicats Rust

`is_active()` répond « strictement `Active` », mais ses appelants posent deux
questions différentes. **On scinde plutôt que d'élargir** — élargir
`is_active()` en place changerait les quatre sites d'un coup, dont deux à
tort :

```rust
pub fn is_active(&self) -> bool { matches!(self, Self::Active) }

/// Est-il encore de l'effectif ? Un journalier l'est : il joue, il porte un
/// maillot, il compte dans la valeur d'équipe. Seul le renvoyé ne l'est plus.
pub fn fait_partie_de_l_effectif(&self) -> bool { !matches!(self, Self::Dismissed) }
```

| Site | Contexte | Verdict |
|---|---|---|
| `player_repository.rs:995` (`find_by_team_id`) | l'effectif rejoué depuis l'event store | **`fait_partie_de_l_effectif()`** |
| `player_dismissed_listener.rs:54` | garde d'idempotence d'un renvoi | **`is_active()`, inchangé** |
| `update_roster_use_case.rs:64` | réordonnancement de l'effectif | **`== Active`, inchangé** |

**Pourquoi `player_repository.rs:995` est le plus grave.** Trois appelants en
dépendent, et chacun casse autrement :

| Appelant | Ce qu'il fait | Sans le journalier |
|---|---|---|
| `team_match_concluded_listener` | pose `MatchConcluded` sur chaque joueur — compteur de matchs, ancre d'historique, récupération BR12 | il ne reçoit rien, alors que l'épic veut qu'il garde ce qu'il a gagné |
| `player_match_impact_listener::load_roster` | dépublication : `revert_team_match_impact` | ses SPP et sa blessure ne sont pas annulés, et le rejeu les réappliquerait par-dessus |
| `player_table_widget::resolve_team_derived` | caractéristiques et SPP restants | il s'affiche sans stats et à zéro SPP |

**Pourquoi le listener de renvoi reste strict.** Un journalier qu'on ne
recrute pas **n'a jamais été embauché** : il ne se renvoie pas, il n'existe
plus. Il n'y a donc pas de renvoi manuel de journalier, et le `DejaSorti` de
ce garde n'est pas une réponse à lui donner. Sa disparition est le sujet de la
carte `456`, par un autre chemin.

Sans cette phrase écrite ici, le prochain lecteur verra une incohérence entre
les trois sites et la « corrigera ».

#### Le contrôle qui referme ça

Le `grep` SQL ne voit rien du Rust. Il en faut deux, et c'est le seul
changement de cette carte qu'aucun test ne rattrape s'il est oublié :

```bash
grep -rn "membership = 'Active'" src/                        # doit être vide
grep -rn "is_active()\|== RosterMembership::Active" src/     # 3 sites, chacun arbitré
```

### 3. `SquadMemberDto` gagne deux champs

```rust
pub is_temporary: bool,                    // membership == Journeyman
pub improvement_label: Option<String>,     // « Blocage », « +1 ST », None
```

**`is_temporary` et non `is_journeyman`** : ce dernier existe déjà sur
`RosterPositionDto` et signifie « ce poste est la ligne journalière du
roster ». Deux homonymes contradictoires dans le même BC seraient une
confusion assurée.

**Il cohabite avec `presence`**, ajouté depuis par la carte 489. Les deux ne se
recouvrent pas et le code doit le dire : `presence` répond « peut-il tenir une
place au prochain match ? », `is_temporary` répond « est-il des nôtres ? ». Un
journalier est `Alignable` et temporaire ; un blessé permanent est `Empeche` et
non temporaire.

**`improvement_label` est un libellé déjà composé**, pas une structure. Il
vient de deux sources, dans cet ordre :

```
acquired_skills[0].skill_name   →  « Blocage »
sinon un delta non nul          →  « +1 ST »
sinon                           →  None
```

**La compétence l'emporte** si les deux existaient — cas impossible
aujourd'hui, un match ne donne pas assez de SPP — parce qu'elle se nomme.

Les deux champs se composent dans `to_squad_member` (`squad_adapter.rs:54`).
**`acquired_skills` et les cinq deltas sont déjà dans `PlayerProjection`** et
déjà dans le `SELECT` de `lire_effectif` : seul `membership` est à ajouter aux
deux.

### 4. Le commentaire de `journeymen_value` — la livraison la plus importante

```rust
// team_value.rs:95
let missing = MATCH_SQUAD_SIZE.saturating_sub(available_count(players));
missing * journeyman_price.0
```

Dès que les journaliers sont de vrais joueurs, `available_count` les compte,
`missing` tombe à zéro, **la fonction rend zéro** — et le résultat reste juste
puisque `players_value` les compte.

**Sans commentaire, quelqu'un la croira morte et la supprimera** — cassant la
valeur d'équipe de **toutes les équipes hors match**, pour lesquelles la
déduction est la seule source. Le LRB l'exige : « les journaliers comptent
toujours dans la Valeur d'Équipe ».

Le commentaire doit dire les deux cas : zéro pendant un match parce qu'ils
existent, la déduction hors match parce qu'ils n'existent pas encore.

## Ce que la carte ne fait pas

- **Aucun écran, aucun événement, aucune migration.** Elle ouvre un socle.
- Elle ne fait pas naître de journalier : c'est la carte `455`.
- Elle ne le fait pas disparaître : c'est la carte `456`.
- Elle ne touche pas au réordonnancement ni au renvoi, dont les prédicats
  restent stricts.

## Tests

| Test | Ce qu'il prouve |
|---|---|
| `l_effectif_inclut_les_journaliers` | intégration, vraie base |
| `l_effectif_exclut_les_renvoyes` | la non-régression du filtre |
| `le_compte_des_disponibles_inclut_les_journaliers` | ligne 174 |
| `les_maillots_pris_incluent_ceux_des_journaliers` | ligne 156 |
| `l_effectif_evenementiel_inclut_les_journaliers` | `player_repository:995`, que le `grep` SQL ne voit pas |
| `from_str_ne_replie_pas_journeyman_sur_active` | le piège du défaut permissif |
| `improvement_label_rend_le_nom_de_la_competence` | « Blocage » |
| `improvement_label_rend_le_delta_a_defaut` | « +1 ST » |
| `improvement_label_prefere_la_competence` | la règle tranchée |
| `journeymen_value_rend_zero_quand_ils_existent` | la collision, documentée par un test |
| `journeymen_value_deduit_hors_match` | l'autre moitié, celle qu'on casserait |

Les deux derniers vont ensemble : ils disent que la fonction a **deux
comportements justes**, ce qu'un lecteur pressé prendrait pour un bug.

Les deux du milieu sont ceux que la version précédente de cette carte n'avait
pas, faute d'avoir vu le filtre Rust et le défaut de `from_str`.

## Checklist

- [ ] La variante `Journeyman`, avec son bras dans `as_str` **et dans `from_str`**
- [ ] `fait_partie_de_l_effectif()` à côté d'`is_active()`, chacun documenté
- [ ] Les **trois requêtes SQL** → `membership <> 'Dismissed'`
- [ ] `player_repository:995` → `fait_partie_de_l_effectif()`
- [ ] Les deux autres sites Rust **laissés stricts**, avec le motif écrit
- [ ] Les **deux `grep`** de contrôle
- [ ] `membership` ajouté à `PlayerProjection` et au `SELECT` de `lire_effectif`
- [ ] `is_temporary` et `improvement_label` sur `SquadMemberDto`, et le
      commentaire qui les distingue de `presence`
- [ ] Le commentaire de `journeymen_value`, disant ses deux cas
- [ ] Les onze tests
- [ ] `make lint && make test && make check-arch`
