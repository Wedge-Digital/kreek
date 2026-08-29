# Tout membre d'une compétition est traité en commissaire

**Priorité : moyenne** — sans effet aujourd'hui, ouvert dès qu'un participant est inscrit
**Dépend de :** rien
**Trouvée par :** la carte 426, en séparant les deux chemins d'autorisation

## Le constat

`find_competition_by_id.sql` alimente les droits de commissaire :

```sql
SELECT c.name AS competition_name, c.logo, cm.coach_id, uc.coach_name
FROM   competitions c
LEFT JOIN competitions_members cm ON cm.competition_id = c.id
LEFT JOIN spaces__user_cache uc ON uc.id = cm.coach_id
WHERE  c.id = $1
```

**Aucun filtre sur `competition_profile`.** Les listes qu'elle produit —
`admin_ids` et `admin_names` — contiennent donc **tous** les membres de la
compétition, quel que soit leur profil. Et `require_admin_access` s'y fie :

```rust
let is_comp_admin = comp_info.admin_ids.contains(&user_id_str)
    || comp_info.admin_names.contains(&coach_name_str);
```

Le nom des champs dit « admin » ; leur contenu dit « membre ».

## Pourquoi c'est sans effet aujourd'hui

`CompetitionProfile` a deux variantes, mais **les deux seuls sites d'insertion**
— `competition_repository.rs:63` (création) et `:242` (mise à jour) — écrivent
`CompetitionAdmin` en dur. `CompetitionUser` n'est jamais écrit : il n'existe
que dans l'enum, sa conversion et ses tests.

Les 3486 lignes de `competitions_members` sont toutes `CompetitionAdmin`, et
aucune compétition n'a plus d'un membre.

## Pourquoi ça compte quand même

`CompetitionUser` **invite explicitement** à inscrire un participant. Le jour où
quelqu'un le fait — et c'est la seule raison d'être de cette variante —, ce
participant devient commissaire de la compétition : il peut modifier le barème,
retirer des poules, attribuer des points manuels.

**Le défaut ne se manifeste pas au moment de la faute.** Le code qui écrira la
ligne sera correct ; c'est la lecture, écrite ailleurs et plus tôt, qui
l'interprétera mal. Ni le compilateur ni les tests ne le diront : aucun test
n'a de `CompetitionUser` à exercer.

## Ce que la carte fait

### Le filtre

```sql
LEFT JOIN competitions_members cm
       ON cm.competition_id = c.id
      AND cm.competition_profile = 'CompetitionAdmin'
```

**Dans le `ON` et non dans le `WHERE`** : c'est un `LEFT JOIN`, et un `WHERE`
sur la table jointe le transformerait en jointure interne — une compétition sans
membre cesserait d'être trouvée, et `find_base_info` rendrait `None` au lieu de
la compétition.

### Le test qui empêche la régression

Un test d'intégration qui **insère un `CompetitionUser`** et vérifie qu'il
n'apparaît ni dans `admin_ids` ni dans `admin_names`. Sans lui, le filtre se
reperdra à la première réécriture de la requête — c'est précisément ce qui est
arrivé la première fois.

**Et sa contre-épreuve** : un `CompetitionAdmin` de la même compétition doit,
lui, apparaître. Sans elle, un filtre qui exclurait tout le monde passerait.

### Le nom des champs

`admin_ids` et `admin_names` deviennent honnêtes une fois le filtre posé — ils
disent alors ce qu'ils contiennent. Aucun renommage n'est nécessaire.

## Ce que la carte ne fait pas

- **Aucune migration** : les 3486 lignes sont déjà toutes `CompetitionAdmin`,
  le filtre ne change rien à leur lecture.
- **Rien sur `CompetitionUser`** : la variante reste inutilisée. Lui donner un
  usage est une autre carte, et c'est elle qui rendrait ce défaut actif.

## Tests

| Test | Ce qu'il prouve |
|---|---|
| `un_membre_non_admin_n_apparait_pas_dans_les_admins` | le filtre |
| `un_admin_apparait_toujours` | la contre-épreuve |
| `une_competition_sans_membre_reste_trouvable` | le `LEFT JOIN` n'est pas devenu interne |

## Checklist

- [x] Le filtre dans le `ON` de `find_competition_by_id.sql`
- [x] Les trois tests d'intégration
- [x] `make lint`, `make check-arch`, `make test`

## Les tests écrits avant le filtre

L'état de départ était exactement celui qu'on veut : les deux gardes passaient,
et **seul le test du défaut échouait**. Il constatait donc le défaut sur le code
d'alors, plutôt que d'être écrit pour convenir au correctif.

Un détour au passage : `CompetitionId` est un ULID, et les fixtures voisines
emploient des « c-zeta » lisibles — elles ne les font jamais passer par le
constructeur du value object. Ces tests-ci l'appellent, donc leurs identifiants
en sont.

## Falsification

| Mutation | Constaté |
|---|---|
| Le filtre déplacé dans le `WHERE` | **2 rouges** |
| Le filtre retiré | 1 rouge : `un_membre_non_admin_n_apparait_pas_dans_les_admins` |

**La première ligne mérite un mot.** Le piège du `LEFT JOIN` n'était pas une
hypothèse d'école : outre le test de garde écrit pour lui, il casse
`test_space_scope::une_competition_n_est_lisible_que_depuis_son_espace` — un
test de portée d'espace, sans rapport apparent, qui dépend lui aussi de ce que
`find_base_info` trouve une compétition. Poser la condition dans le `WHERE`
aurait donc rendu `404` des compétitions existantes, sur des chemins qu'on ne
pense pas à relire en touchant à une jointure.

## Ce qui reste vrai après le correctif

`CompetitionUser` n'est toujours écrit nulle part : la variante reste inutilisée,
et les 3486 lignes de `competitions_members` restent toutes `CompetitionAdmin`.
Le filtre ne change donc **rien** au comportement d'aujourd'hui — il rend la
variante utilisable sans danger le jour où quelqu'un s'en servira.
