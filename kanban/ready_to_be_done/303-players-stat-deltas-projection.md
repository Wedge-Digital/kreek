# BC `players` — Deltas de caractéristiques en projection

**Priorité : haute**
**Dépend de :** `302-players-customisation-domain.md`
**Contexte :** `players` — projection

## Objectif

Faire porter à `players_proj` le **cumul des deltas** par caractéristique,
toutes sources confondues, recalculé depuis l'agrégat à chaque écriture.

**Spec :** `docs/specs/player-customisation/player-detail/07-integration.md`.

---

## La carte la plus risquée de la série

Elle touche des comportements **existants** : augmentations achetées en SPP,
séquelles de blessure, et corrections de match. C'est la seule de la série qui
puisse casser quelque chose qui marche aujourd'hui.

Elle est isolée pour ça — son propre commit, ses propres tests, un `git revert`
propre si elle tourne mal. Elle ne dépend que des événements de la 302 et peut
donc partir avant que le panier existe.

## Migration

```sql
ALTER TABLE players_proj
    ADD COLUMN ma_delta SMALLINT NOT NULL DEFAULT 0,
    ADD COLUMN st_delta SMALLINT NOT NULL DEFAULT 0,
    ADD COLUMN ag_delta SMALLINT NOT NULL DEFAULT 0,
    ADD COLUMN pa_delta SMALLINT NOT NULL DEFAULT 0,
    ADD COLUMN av_delta SMALLINT NOT NULL DEFAULT 0;
```

Signées : une séquelle ou une dégradation les rend négatives.

## Recalcul, jamais incrément

`upsert_player_projection(tx, event)` ne reçoit que la transaction et
l'événement. Il relit, **dans la même transaction**, les événements du joueur
et reconstruit l'agrégat :

```
append de l'événement
  → SELECT payload FROM players_events WHERE player_id = … ORDER BY version
  → Player::from_events(&events)
  → cumul par caractéristique
  → UPDATE players_proj SET *_delta = …
```

**Aucun port n'entre dans la transaction**, et c'est le choix des *deltas* qui
le permet : le cumul ne dépend que des événements du joueur, jamais du
catalogue de postes. Une projection de valeurs absolues aurait exigé la base du
poste, donc `references`.

**`MatchImpactReverted` est traité gratuitement** : `Player::from_events` sait
déjà le défaire, c'est sa raison d'être. Le rejeu, l'ordre et les corrections
de match sont couverts sans code dédié — alors que c'était le point délicat
identifié en conception.

Coût assumé : un `SELECT` et un fold par append, sur quelques dizaines de
lignes.

## Lecture

`projection_repository` expose les cinq deltas. `resolve_stats` reste la voie
de la fiche joueur ; les consommateurs qui n'ont besoin que du delta peuvent
désormais le lire en SQL.

---

## Checklist

- [ ] Migration des cinq colonnes
- [ ] Recalcul du cumul dans `upsert_player_projection`, dans la transaction
- [ ] Les cinq deltas exposés par `projection_repository`
- [ ] Test repository : une augmentation SPP écrit le delta attendu
- [ ] Test repository : une séquelle écrit un delta **négatif**
- [ ] Test repository : `MatchImpactReverted` **ramène** le delta à sa valeur
      d'avant match — le test qui justifie le recalcul
- [ ] Test repository : une customisation de caractéristique écrit son offset
- [ ] Test repository : deux sources cumulent (séquelle + augmentation SPP)
- [ ] Test : la projection est identique après rejeu complet des événements
- [ ] `make test` complet — la carte touche des chemins existants
