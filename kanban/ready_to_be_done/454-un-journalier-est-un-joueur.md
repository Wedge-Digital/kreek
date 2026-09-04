# Un journalier est un joueur

**Épic :** E15 — Recruter un journalier
**Ordre :** 1 · **Dépend de :** rien
**Conception :** `docs/specs/embaucher-un-journalier/` (`00-conception.md`,
`ecran-de-recrutement/03-back.md`)

## Objectif

Ouvrir un troisième statut d'appartenance, et faire en sorte que **quatre
lectures cessent d'exclure les journaliers**. Aucun écran, aucun événement.

## Pourquoi elle est seule — et la plus risquée de la série

Elle change **quatre requêtes SQL qu'aucun compilateur ne vérifie**, sur une
projection que quatre écrans lisent. Une erreur là-dessus se voit partout et se
diagnostique mal : un journalier invisible ne produit aucune erreur, seulement
un nombre faux au rapport suivant.

## Conception

### 1. La troisième variante

```rust
pub enum RosterMembership { Active, Journeyman, Dismissed }
```

**Aucune migration de données** : la variante s'ajoute, les 49 262 lignes
`Active` et les 104 `Dismissed` ne bougent pas.

**À vérifier** : si `players_proj.membership` porte une contrainte `CHECK`, elle
doit accepter `'Journeyman'`. Sinon la colonne est un `TEXT` et rien n'est dû.

Un seul site filtre dessus en Rust — `update_roster_use_case:64` — et il garde
`== Active` : réordonner l'effectif ne concerne pas les journaliers, ils partent
ou deviennent permanents.

### 2. Les quatre requêtes — le cœur de la carte

| Fichier | Ligne | Ce qu'elle sert | Le journalier doit y figurer |
|---|---|---|---|
| `players/io/repository/projection_repository.rs` | 29 | l'effectif complet | il est visible pendant le match |
| idem | 130 | les maillots pris | il en porte un |
| idem | 148 | le compte des disponibles | sinon on recrée des journaliers pour combler des journaliers |
| `infrastructure/teams/squad_adapter.rs` | 47 | l'effectif vu par `teams` | il compte dans la valeur d'équipe |

**Les quatre répondent oui.** Le filtre devient :

```sql
WHERE team_id = $1 AND membership <> 'Dismissed'
```

**Et non `IN ('Active','Journeyman')`** : la liste devrait être tenue à jour à
chaque nouvelle variante, alors que la question posée est bien « ce joueur
fait-il encore partie de l'effectif ? ».

**Le contrôle qui referme ça** :

```bash
grep -rn "membership = 'Active'" src/   # doit ne rien rendre
```

Une ligne de checklist vaut mieux qu'un espoir. C'est le seul changement de
cette carte qu'aucun test ne rattrapera s'il est oublié.

### 3. `SquadMemberDto` gagne deux champs

```rust
pub is_temporary: bool,                    // membership == Journeyman
pub improvement_label: Option<String>,     // « Blocage », « +1 ST », None
```

**`is_temporary` et non `is_journeyman`** : ce dernier existe déjà sur
`RosterPositionDto` et signifie « ce poste est la ligne journalière du roster ».
Deux homonymes contradictoires dans le même BC seraient une confusion assurée.

**`improvement_label` est un libellé déjà composé**, pas une structure. Il vient
de deux sources, dans cet ordre :

```
acquired_skills[0].skill_name   →  « Blocage »
sinon un delta non nul          →  « +1 ST »
sinon                           →  None
```

**La compétence l'emporte** si les deux existaient — cas impossible aujourd'hui,
un match ne donne pas assez de SPP — parce qu'elle se nomme.

La requête de `squad_adapter.rs:44` gagne donc `membership`, `acquired_skills`
et les cinq deltas. Mesuré : 298 joueurs sur 49 000 ont une compétence acquise,
47 une caractéristique améliorée.

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

## Tests

| Test | Ce qu'il prouve |
|---|---|
| `l_effectif_inclut_les_journaliers` | intégration, vraie base |
| `l_effectif_exclut_les_renvoyes` | la non-régression du filtre |
| `le_compte_des_disponibles_inclut_les_journaliers` | ligne 148 |
| `les_maillots_pris_incluent_ceux_des_journaliers` | ligne 130 |
| `improvement_label_rend_le_nom_de_la_competence` | « Blocage » |
| `improvement_label_rend_le_delta_a_defaut` | « +1 ST » |
| `improvement_label_prefere_la_competence` | la règle tranchée |
| `journeymen_value_rend_zero_quand_ils_existent` | la collision, documentée par un test |
| `journeymen_value_deduit_hors_match` | l'autre moitié, celle qu'on casserait |

Les deux derniers vont ensemble : ils disent que la fonction a **deux
comportements justes**, ce qu'un lecteur pressé prendrait pour un bug.

## Checklist

- [ ] La variante `Journeyman`, et le `CHECK` vérifié
- [ ] **Les quatre requêtes**, puis `grep -rn "membership = 'Active'" src/` vide
- [ ] `is_temporary` et `improvement_label` sur `SquadMemberDto`
- [ ] Le commentaire de `journeymen_value`, disant ses deux cas
- [ ] Les neuf tests
- [ ] `make lint && make test && make check-arch`
