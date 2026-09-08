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

## R28 — l'appelant dit qui répond, l'agrégat vérifie

`Repondant` vaut `Jeton | Coach(CoachId) | Organisateur(CoachId)`. Cet écran-ci
passe `Organisateur(id du connecté)` ; la route publique passera `Jeton`, et
l'encart `Coach(id du connecté)`.

Le use case ne contrôle rien : il transmet, et l'agrégat refuse un `Coach(id)`
qui ne correspond pas au `coach_id` de la réponse (carte 512). C'est la règle
« le use case fournit les faits, le domaine décide », appliquée à l'identité.

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

- [x] `RecordAnswerCommand` avec `Venue` et `Repondant` typés — trois variantes
- [x] Le use case, instrumenté, son enum d'erreur
- [x] `EtatJournee` composé des deux sources
- [x] Tests unitaires : les cinq de la liste, plus R16, R7/R28 et la journée sans
      campagne — 10 tests
- [x] `make lint`, `make check-arch`, `make test` — 1827/1827

## Ce que la réalisation a tranché

**`rencontre_a_refaire` est un `Option<PairingId>`, pas un `bool`.**
`EffetReponse::EnregistreeRencontreARefaire { pairing }` porte déjà
l'identifiant ; le réduire à un booléen jetterait la seule information utile, et
la carte 518 devrait la redécouvrir pour proposer de refaire *cette* rencontre-là.
Le principe ne change pas : le use case signale, il ne répare pas.

**Toujours `None` sur une arrivée tardive** (R16) : un arrivant n'apparaît dans
aucune rencontre, donc aucune n'est à refaire. C'est `desaccord` qui le fait
voir, et l'écran le recalcule à chaque affichage — le bénéfice de R24.

**`etat_de_la_journee` a déménagé** de `reopen_survey_use_case.rs` vers
`presences/etat_journee.rs`. Elle y avait été publiée en 515 en annonçant que 516
et 517 la reprendraient ; faire importer un use case depuis un autre est un
couplage sans raison d'être. Déplacement au sens strict de la règle 5 —
copier-coller exact, seuls les imports changent, et le `// arch:no-instrument`
part avec elle.

Le test de la fonction a suivi, et deux lui ont été ajoutés, qui manquaient :

- **un seul rapport publié fige la journée entière** — ce n'est pas la rencontre
  qui se verrouille mais la journée, parce qu'un nouveau tirage la
  redistribuerait tout entière ;
- **une journée vide n'est jamais figée**, le port recevant une liste vide.
