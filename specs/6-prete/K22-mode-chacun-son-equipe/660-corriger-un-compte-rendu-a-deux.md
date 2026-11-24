---
id: 660-corriger-un-compte-rendu-a-deux
parent: K22
status: approved
---

# Corriger un compte rendu à deux

**Épic :** E22 — Le mode « Chacun son équipe » · **Ordre :** 11 · **Dépend de :** 659-C  
**Dépend de** : 659-A-valider-son-camp, 659-C-le-mode-chacun-son-equipe-s-ouvre
**Conception :** `docs/specs/saisie-mobile/E22-cadrage.md` (R10, § 1 « La correction »)

## Le constat

- La dépublication suit `correction_eligibility_service::evaluate` : bloquée si
  des SPP ont été dépensés depuis le match ou si une équipe est en phase
  d'amélioration (`src/app/match_report/use_cases/correction_eligibility_service.rs:22-37`,
  verdict `:59`). `unpublish` rend un `ReadyToPublish` marqué
  `was_published_before` (`domain/match_report_published.rs:86-100`).
- La garde est `authorize_recap_action` (`io/web/recap_controller.rs:480-498`),
  règle `can_write_report` : en mode B, depuis la 652, elle laisse passer les
  deux coachs.
- **La correction par camp existe déjà** : en `ReadyToPublish`, le fan factor,
  les actions et le MVP passent par `into_pre_match()` et la garde par camp
  (652 à 657), et l'après-match d'un camp « remplace ses seules valeurs »
  (`domain/team_post_match.rs`, 658). Reste ce que la 659-A a ajouté : des
  validations, que la dépublication ne remet pas à zéro.
- Le récapitulatif dépublié affiche un bandeau « Il a été dépublié… »
  (`io/web/templates/recap.html:211-216`), sans dire par qui.

## Ce qui change

- **`MatchReportUnpublished`** remet les deux validations à zéro, à la commande
  comme au rejeu (`domain/match_report_state.rs`, arête `Published` →
  `ReadyToPublish`) : aucun événement nouveau.
- Chaque coach corrige son camp — par les écrans existants — puis revalide ;
  la seconde validation republie (659-A).
- Le récapitulatif corrigeable en mode B : « Dépublié par … — à revalider par
  les deux camps », le bloc des validations à zéro (widget de la 659-C).
- Mode A inchangé.

## Fichiers touchés

`domain/match_report_published.rs`, `domain/match_report_ready_to_publish.rs`,
`domain/match_report_state.rs`, `io/web/recap_controller.rs`,
`io/web/templates/recap.html` (chemins sous `src/app/match_report/`) ; le
fichier e2e de la 659-A, `tests/impact-map.toml`.

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
