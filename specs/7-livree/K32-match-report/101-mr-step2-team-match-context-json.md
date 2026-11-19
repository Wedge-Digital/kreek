# MR-STEP2-05 — Endpoint JSON team-match-context (BC Teams)

## Objectif

Endpoint JSON fournissant les données d'équipe nécessaires à la page step 2 : dedicated fans, player count, CTV, treasury, journeyman type.

## Dépendances

Aucune (indépendant du BC match_report).

## Fichiers

- `src/app/teams/io/web/widgets/team_match_context_widget.rs` (nouveau)
- `src/app/teams/io/web/widgets/mod.rs`
- `src/app/teams/routes.rs`
- `src/app/teams/router.rs`

## Conception

Voir `docs/specs/match-report/step2-pre-match/04-dtos.md` (struct `TeamMatchContextJson`)

## Checklist

- [ ] Struct `TeamMatchContextJson` (Serialize)
- [ ] Handler `get_team_match_context_json(Path(space_id), Query(team_id), State)`
- [ ] Route `TEAM_MATCH_CONTEXT_JSON`
- [ ] Vérifier que la projection Teams expose tous les champs nécessaires — sinon étendre
