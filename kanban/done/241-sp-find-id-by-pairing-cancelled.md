# `match_report` — `find_id_by_pairing` ignore les rapports annulés

**Priorité : basse**
**Dépend de :** rien (indépendante des cartes 238-240)
**Fichiers :** `src/app/match_report/io/repository/match_report_repository.rs`

## Objectif

Asymétrie entre deux requêtes voisines de `match_report_proj` :

```sql
-- find_id_by_round_and_teams
WHERE round_id = $1 AND phase != 'Cancelled' AND (…)

-- find_id_by_pairing
WHERE pairing_id = $1                      -- ← aucun filtre de phase
```

Conséquence : un rapport annulé reste résoluble depuis son pairing. La route
`/app/{space}/match-report/pairing/{pairing_id}` renvoie alors un `410 GONE`
(`edit_match_report` sur un état `Cancelled`) au lieu du `404` attendu.

Peu visible aujourd'hui — le pairing est supprimé en même temps que le rapport
est annulé, donc la ligne qui portait le lien a disparu. Mais la carte 238
multiplie les rapports annulés, et cette requête sert aussi de garde
anti-doublon dans `pairing_created_listener`.

## Conception

Ajouter `AND phase != 'Cancelled'` à `find_id_by_pairing`, par symétrie avec
`find_id_by_round_and_teams`.

Vérifier au passage les autres consommateurs de la méthode
(`pairing_deleted_listener`, `from_pairing`, `pairing_created_listener`) : aucun
n'a de raison de vouloir retrouver un rapport annulé.

## Checklist

- [ ] `AND phase != 'Cancelled'` dans `find_id_by_pairing`
- [ ] Consommateurs relus, aucun ne dépend du comportement actuel
- [ ] Test d'intégration repository : un rapport annulé n'est plus retrouvé par son pairing
- [ ] `make test` passe
- [ ] `make check-arch` passe
