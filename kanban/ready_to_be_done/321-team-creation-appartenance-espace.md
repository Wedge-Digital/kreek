# BC `team_creation` — cloisonnement des espaces

**Priorité : moyenne**
**Dépend de :** `316` (audit)
**Contexte :** `team_creation` — 25 routes

---

## Non sondé

Un seul fichier de `team_creation/io/web/` mentionne `space_id`, sans
comparaison. La sonde reste à faire.

## Les ressources

`team_drafts` et `team_roster_selections` portent toutes deux `space_id` :
**comparaison directe**, pas de saut.

## Un point de vigilance propre à ce BC

C'est ici que vivent les transactions de la carte 317 —
`team_roster_selections` était la table visible dans la transaction fantôme.
Migrer ces handlers veut dire les relire : si l'un d'eux ouvre une transaction
sur un chemin qui peut être annulé, c'est l'occasion de le noter, même si sa
correction relève de la 317.


## Le patron

`src/app/team_creation/io/web/space_scope.rs`, sur le modèle exact de la carte 315 :

```rust
pub async fn charger_brouillon_de_l_espace(
    state: &AppState, space_id: &str, id: &str,
) -> Result<TeamDraft, Response>
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

- [ ] Sonde de lecture préalable
- [ ] `space_scope.rs` : gardes brouillon et sélection de roster (directs)
- [ ] Les 25 routes migrées
- [ ] Vérifié : plus aucun chargement direct par identifiant dans `io/web/`
- [ ] Sonde consignée
- [ ] Scénario e2e
- [ ] Noter tout handler ouvrant une transaction sur un chemin annulable (carte 317)
