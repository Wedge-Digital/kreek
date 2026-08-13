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

## Tests

Le contrôle porte sur un `AppState` : **pas de test unitaire possible** faute du
harnais de la carte 311. C'est le troisième report du même genre après les
cartes 308 et 315.

Couverture attendue :

- une sonde manuelle consignée dans la carte, comme en 315 ;
- un scénario e2e : lecture **et** écriture croisées → `404`, cas nominal
  inchangé.

## Checklist

- [ ] `space_scope.rs` : garde article (direct) et garde commentaire (saut)
- [ ] Les 5 routes migrées
- [ ] Vérifié : plus aucun chargement direct par identifiant dans `io/web/`
- [ ] Sonde consignée : le commentaire croisé rend désormais `404`
- [ ] Scénario e2e
