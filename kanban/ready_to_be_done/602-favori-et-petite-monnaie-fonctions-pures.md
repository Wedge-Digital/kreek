# Le favori et la petite monnaie deviennent des fonctions pures

**Priorité : moyenne**
**Épic :** E16 — Imprimer la feuille d'équipe · PR 2
**Dépend de :** rien
**Spec :** `06-domaine.md` (`underdog.rs`, R10, R11)
**Fichiers :** `src/app/match_report/domain/underdog.rs`, `src/app/match_report/domain/match_report_pre_match.rs`, `src/app/match_report/domain/mod.rs`

## Objectif

La feuille de match imprime le favori et la petite monnaie. Ce calcul vit dans
l'agrégat `MatchReportPreMatch`, sur des champs qu'il ne remplit qu'une fois le
rapport commencé. L'impression a besoin de la **même** règle, sur deux VE
quelconques.

## Conception

- `top_dog(home_tv, away_tv) -> Side` et `tv_gap(home_tv, away_tv) -> u32`,
  extraits de `topdog_team_id` et `petty_cash` **par copier-coller** (règle 5),
  puis adaptés pour prendre des `TeamValue`.
- L'agrégat les appelle : `topdog_team_id` et `petty_cash` gardent exactement
  leur comportement, `expect` compris (hors périmètre).
- Aucune règle nouvelle : à égalité, l'équipe à domicile.

## Checklist

- [ ] `underdog.rs` + tests : plus forte à domicile · à l'extérieur · égalité ; écart dans les deux sens · nul
- [ ] L'agrégat délègue ; ses tests existants restent verts sans modification
- [ ] `make check-arch`, `make lint`, `make test`
