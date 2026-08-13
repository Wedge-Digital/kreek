# BC `teams` — cloisonnement des espaces

**Priorité : haute** — fuite prouvée en lecture
**Dépend de :** `316` (audit)
**Contexte :** `teams` — 29 routes

---

## Prouvé en lecture

Équipe « Les gros bourrins » appartenant à l'espace *Bordeaux BBC*, demandée
depuis l'espace *E2E* :

```
GET /app/<espace E2E>/teams/<équipe de Bordeaux>
→ 200, le nom réel de l'équipe est servi
```

## L'écriture reste non prouvée — et c'est à retenir

L'ajout au panier de recrutement de cette même équipe a rendu `422`. Mais
l'équipe était en phase `ReadyToPlay`, donc **c'est vraisemblablement le garde
de phase qui a parlé, pas l'autorisation**.

La sonde est donc à refaire sur une équipe réellement en phase de recrutement.
Ne pas conclure de ce `422` que l'écriture est protégée : rien dans le code ne
le suggère.

## La ressource

`team_proj` porte `space_id` : **comparaison directe**.

Trois fichiers de `teams/io/web/` mentionnent `space_id`, mais **aucune
comparaison** — vérifié. Ce sont des passages de paramètre, pas des contrôles.


## Le patron

`src/app/teams/io/web/space_scope.rs`, sur le modèle exact de la carte 315 :

```rust
pub async fn charger_equipe_de_l_espace(
    state: &AppState, space_id: &str, id: &str,
) -> Result<Team, Response>
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

- [ ] Sonde d'écriture sur une équipe en phase de recrutement, pour lever le doute
- [ ] `space_scope.rs` : garde équipe (direct)
- [ ] Les 29 routes migrées
- [ ] Vérifié : plus aucun chargement direct par identifiant dans `io/web/`
- [ ] Sondes consignées : lecture **et** écriture croisées → `404`
- [ ] Scénario e2e
