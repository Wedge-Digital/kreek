# `competitions` — Compensation d'une dépublication

**Priorité : haute**
**Dépend de :** `231-mrc-publisher-app-events.md`
**Fichiers :** `src/app/competitions/io/app_events/match_report_unpublished_listener.rs` (nouveau), `src/app/competitions/context.rs`
**Spec :** `docs/specs/match-report-correction/recap/03-back.md`, `07-integration.md`

## Objectif

Remettre la ligne de résultats/calendrier dans son état d'avant publication.

## Conception

Listener souscrivant à l'app event bus, filtrant
`MatchReportAppEvent::MatchReportUnpublished`.

```sql
UPDATE competition_match_display_proj
   SET match_status      = 'in_progress',
       home_score        = NULL,
       away_score        = NULL,
       home_casualties   = NULL,
       away_casualties   = NULL,
       match_report_url  = $2
 WHERE pairing_id = $1
```

`match_report_url` pointe vers l'**édition** du rapport
(`AppRoutes::default().match_report.edit_match_report(...)`), symétrique de ce
que fait `match_report_confirmed_listener`.

`pairing_id` vient du payload : aucune requête de résolution supplémentaire.

### Ne recrée aucun pairing

Point de vigilance principal. `resolve_pairing_id` du listener de publication
crée un pairing pour les rapports manuels. **La compensation ne doit rien
recréer** : le pairing existe déjà, et le rejeu à la re-publication le
retrouvera via `payload.pairing_id`.

Si `pairing_id` est `None` dans le payload, il n'y a rien à compenser — sortir
sans rien faire, sans log d'erreur.

### Idempotence

`UPDATE` à valeurs absolues sur une clé stable : naturellement idempotent
(règle 11). Aucune garde supplémentaire.

### Câblage

`competitions::context::init_listeners` — déjà appelé depuis `main.rs`, aucun
changement à y faire.

## Checklist

- [ ] Listener créé, filtrant `MatchReportUnpublished`
- [ ] `UPDATE` remettant statut, scores, sorties et URL
- [ ] Aucun pairing recréé
- [ ] `pairing_id` absent → sortie silencieuse
- [ ] Câblé dans `context.rs`
- [ ] Test d'intégration : après compensation, la ligne est `in_progress` et les scores nuls
- [ ] Test d'intégration : deux compensations successives donnent le même résultat
- [ ] Test : aucun pairing supplémentaire créé pour un rapport manuel
- [ ] `make test` passe
- [ ] `make check-arch` passe
