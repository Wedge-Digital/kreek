# BC `news` — cloisonnement des espaces

**Priorité : moyenne** — écriture prouvée, mais bénigne
**Dépend de :** `316` (audit)
**Contexte :** `news` — 5 routes

---

## Prouvé en écriture

C'est sur ce BC que l'écriture croisée a été démontrée :

```
GET  /app/<espace E2E>/home/articles/<article d'un autre espace>
→ 200, le titre réel est servi

POST …/comments   { content: "sonde-audit-316" }
→ 200, la ligne est écrite en base
```

Le commentaire de sonde a été supprimé, base rendue à son état initial.

## Pourquoi en dernier malgré la preuve

C'est le seul BC dont l'abus ne touche pas les données de jeu : un commentaire
indu se supprime, un prix ou un score customisé se propage.

Le rang dit la gravité, pas la certitude — c'est ici que la preuve est la plus
nette.

## Les ressources, et le saut

| Ressource | Espace |
|---|---|
| `articles` | colonne `space_id` — comparaison directe |
| `comments` | **aucune colonne** → saut par `article_id` |


## Le patron

`src/app/news/io/web/space_scope.rs`, sur le modèle exact de la carte 315 :

```rust
pub async fn charger_article_de_l_espace(
    state: &AppState, space_id: &str, id: &str,
) -> Result<Article, Response>
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

- [ ] `space_scope.rs` : garde article (direct) et garde commentaire (saut)
- [ ] Les 5 routes migrées
- [ ] Vérifié : plus aucun chargement direct par identifiant dans `io/web/`
- [ ] Tests de handler : matrice d'appartenance (croisé/nominal, lecture/écriture)
- [ ] Un scénario e2e de bout en bout
