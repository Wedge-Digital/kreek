# Corriger un compte rendu à deux

**Épic :** E22 — Le mode « Chacun son équipe » · **Ordre :** 9 · **Dépend de :** 659
**Conception :** `docs/specs/saisie-mobile/E22-cadrage.md` (R10, § 1 « La correction »)

## Le constat

- La dépublication suit `correction_eligibility_service::evaluate` : bloquée si
  des SPP ont été dépensés depuis le match ou si une équipe est en phase
  d'amélioration (`src/app/match_report/use_cases/correction_eligibility_service.rs:19-58`).
  `unpublish` rend un `ReadyToPublish` marqué `was_published_before`
  (`domain/match_report_published.rs:81-95`).
- La garde est `can_write_report` (`io/web/recap_controller.rs:477-508`) : en
  mode B, depuis la 652, elle laisse passer les deux coachs.
- Rien ne retient de validation à remettre à zéro avant la 659.

## Ce qui change

- **`MatchReportUnpublished`** remet les deux validations à zéro, à la commande
  comme au rejeu (`domain/match_report_state.rs`, arête `Published` →
  `ReadyToPublish`) : aucun événement nouveau.
- Chaque coach corrige son camp — fan factor, actions, MVP, après-match — puis
  revalide ; la seconde validation republie (659).
- Le récapitulatif corrigeable en mode B : « Dépublié par … — à revalider par
  les deux camps », l'état des validations à zéro.
- Mode A inchangé.

## Fichiers touchés

`domain/match_report_published.rs`, `domain/match_report_ready_to_publish.rs`,
`domain/match_report_state.rs`, `io/web/recap_controller.rs`,
`templates/recap.html`.

## Règles et invariants

- L'un ou l'autre coach dépublie, aux règles actuelles d'éligibilité (R10).
- La republication demande de nouveau les deux validations, même au coach qui
  n'a rien changé.
- Les compensations de `teams`, `ranking`, `competitions` et `players` sont
  celles d'aujourd'hui.

## Tests

- **Unitaires** : dépublication d'un rapport mode B → aucune validation ; rejeu
  de trois cycles publier / dépublier ; éligibilité bloquée inchangée.
- **E2E** : deux contextes ; le visiteur dépublie, le domicile voit le rapport
  redevenir corrigeable et ses validations à zéro ; chacun revalide, la seconde
  republie ; le classement suit.

## Conséquences sur les données de production

Aucune donnée modifiée, aucun événement ajouté. L'éligibilité à la correction
est inchangée ; les rapports en mode A se corrigent comme aujourd'hui.

## Terminé quand

Un compte rendu publié en mode B, dépublié par l'un ou l'autre coach, ne se
republie qu'après les deux nouvelles validations.
