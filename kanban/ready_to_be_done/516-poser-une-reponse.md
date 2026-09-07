# Poser une réponse

**Priorité : haute — le point d'écriture qu'appellent les trois unités**
**Épic :** E16 — Sondage de présence
**Dépend de :** 510, 512
**Fichiers :** `src/app/competitions/use_cases/presences/record_answer_use_case.rs`

## L'objectif

Un seul use case pour les trois chemins d'entrée : l'organisateur depuis les
boutons de la carte, le coach depuis son jeton, le coach connecté depuis
l'encart.

R1 porte sur l'équipe, jamais sur le coach ; **un use case par chemin d'entrée
aurait fait trois endroits où tenir R6 et R13.**

## Conception

```rust
pub async fn execute(
    cmd: RecordAnswerCommand,
    survey_repo: &dyn IPresenceSurveyRepository,
    match_day_repo: &dyn IMatchDayRepository,
    match_report_port: &dyn IMatchReportStatusPort,
) -> Result<AnswerOutcome, RecordAnswerError>
```

1. charge l'agrégat par `round_id` — `SurveyNotFound`
2. compose `EtatJournee` : `find_published_pairings` pour `figee` (R13), les
   appariements de la journée pour `rencontres` (R24)
3. `survey.enregistrer(...)` — l'agrégat refuse R19, R13, R21
4. persiste
5. rend `AnswerOutcome { rencontre_a_refaire: bool }`

**Le use case n'interroge pas le port trois fois pour trois questions.** Il
charge les faits une fois et les passe ; l'agrégat décide.

**Il ne répare rien.** Quand la journée est déjà appariée, il enregistre et
signale qu'une rencontre est à refaire — c'est la carte 518 qui répare, sur
décision de l'organisateur.

## `Repondant` est un champ de la commande

Jamais une déduction du handler. Si l'auteur se déduisait de la route, la page
publique et l'encart devraient chacun le reconstituer, et R6 tiendrait à trois
endroits au lieu d'un.

## R13 par le port, jamais par la projection locale

`competitions` porte pourtant `home_score` et `away_score` dans ses propres DTOs
de journée — la tentation est de lire ce qu'on a sous la main. Mais cette
projection est alimentée par un app event, donc **en retard d'un battement**, et
R13 est un garde-fou bloquant : la fraîcheur y est critique. C'est le critère du
CLAUDE.md — consultation bloquante, port synchrone, jamais cache local.

## Checklist

- [ ] `RecordAnswerCommand` avec `Venue` et `Repondant` typés
- [ ] Le use case, instrumenté, son enum d'erreur
- [ ] `EtatJournee` composé des deux sources
- [ ] Tests unitaires : R6 (l'organisateur pose une réponse, l'auteur est gardé) ·
      R12 (la sortie signale la rencontre à refaire) ·
      R13 (journée figée refusée) · R19 (équipe hors campagne) ·
      R21 (campagne close : coach refusé, organisateur accepté)
- [ ] `make lint`, `make check-arch`, `make test`
