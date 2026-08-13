# BC `match_report` — cloisonnement des espaces

**Priorité : haute** — aucune vérification, données de jeu
**Dépend de :** `316` (audit)
**Contexte :** `match_report` — 23 routes

---

## Non sondé, et pourtant classé haut

Aucune sonde n'a été tentée sur ce BC : un rapport de match est une donnée de
jeu, et une sonde d'écriture y aurait laissé des traces difficiles à défaire.

Le classement en priorité haute repose sur la lecture du code :
**aucun fichier de `match_report/io/web/` ne mentionne l'espace d'une
ressource**, exactement comme `competitions`, où la fuite a été prouvée.

C'est une déduction, pas une preuve. La sonde reste à faire au début de la
carte, sur une lecture — sans risque — pour confirmer avant de corriger.

## La ressource

`match_report_proj` porte `space_id` : **comparaison directe**, pas de saut.


## Le patron

`src/app/match_report/io/web/space_scope.rs`, sur le modèle exact de la carte 315 :

```rust
pub async fn charger_match_report_de_l_espace(
    state: &AppState, space_id: &str, id: &str,
) -> Result<MatchReport, Response>
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

- [ ] Sonde de lecture préalable, pour confirmer la déduction
- [ ] `space_scope.rs` : garde rapport de match (direct)
- [ ] Les 23 routes migrées
- [ ] Vérifié : plus aucun chargement direct par identifiant dans `io/web/`
- [ ] Sonde consignée : lecture croisée → `404`, nominal → `200`
- [ ] Scénario e2e
