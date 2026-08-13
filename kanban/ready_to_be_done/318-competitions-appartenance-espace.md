# BC `competitions` et `ranking` — cloisonnement des espaces

**Priorité : haute** — fuite prouvée, données de jeu
**Dépend de :** `316` (audit)
**Contexte :** `competitions` (44 routes), `ranking` (2 routes)

---

## Prouvé

Compétition « Ligue Open » appartenant à l'espace *Bordeaux BBC*, demandée
depuis l'espace *E2E* dont l'appelant est admin :

```
GET /app/<espace E2E>/competitions/<comp de Bordeaux>/<saison>
→ 200, le nom réel de la compétition est servi
GET …/standings
→ 200
```

Aucun fichier de `competitions/io/web/` ne mentionne l'espace d'une ressource.

## Les deux ressources, et le saut

| Ressource | Espace |
|---|---|
| `competitions` | colonne `space_id` — comparaison directe |
| `competition_seasons` | **aucune colonne** → saut par `competition_id` |

Une saison n'a pas d'espace en propre, elle en hérite : le saut est préféré à
l'ajout d'une colonne, qui créerait une seconde source de vérité.

## `ranking` voyage avec

Ses deux routes portent `{competition_id}` et `{season_id}` — même compétition,
même saut, et `ranking_lines` n'a pas non plus de `space_id`. Le migrer
séparément voudrait dire écrire deux fois le même saut.


## Le patron

`src/app/competitions/io/web/space_scope.rs`, sur le modèle exact de la carte 315 :

```rust
pub async fn charger_competition_de_l_espace(
    state: &AppState, space_id: &str, id: &str,
) -> Result<Competition, Response>
```

Charge, compare l'espace de la ressource à celui du chemin, rend **`404`** en
cas de divergence — jamais `403`, qui confirmerait l'existence d'une ressource
d'un autre espace à qui l'énumère.

Le contrôle vient **avant** l'autorisation : il ne s'agit pas de savoir qui a le
droit, mais de quoi on parle.

## La règle qui fait la solidité

Le garde doit devenir le **seul** moyen d'obtenir la ressource depuis
`io/web/` du BC. La carte 315 a montré que c'est vérifiable : après migration,
plus aucun chargement direct par identifiant ne doit subsister dans cette
couche. C'est cette vérification, et non le nombre de handlers migrés, qui dit
que la carte est finie.

## Tests — plus de report

Le harnais de la carte 311 existe : `web::test_harness::Harnais` monte le
routeur de production, se connecte par le vrai parcours, et rejoue son cookie.
Le patron est celui de `players/io/web/tests/test_space_scope.rs`.

**Tests de handler, obligatoires :**

- lecture croisée → `404`, **et** lecture nominale → `200`. C'est l'écart qui
  prouve, pas le `404` seul : une assertion qui ne vérifierait que le refus
  passerait tout aussi bien si la ressource n'existait pas ;
- écriture croisée → `404`, écriture nominale → autre chose que `404` ;
- identifiant d'espace mal formé → `400`.

**Un scénario e2e**, et un seul : le parcours de bout en bout en navigateur. La
matrice d'autorisation n'y a plus sa place — elle coûte des minutes là où elle
coûte des millisecondes ici.

## Checklist

- [ ] `space_scope.rs` : garde compétition (direct) et garde saison (saut)
- [ ] Les 44 routes de `competitions` migrées
- [ ] Les 2 routes de `ranking` migrées
- [ ] Vérifié : plus aucun chargement direct par identifiant dans `io/web/` des deux BCs
- [ ] Tests de handler : matrice d'appartenance (croisé/nominal, lecture/écriture)
- [ ] Un scénario e2e de bout en bout
