# Écrire le tirage au calendrier

**Épic :** E18 — Appariement des présents · **Ordre :** 7 · **Dépend de :** 600
**Conception :** `docs/specs/appariement-des-presents/calendrier/04-dtos.md` (§ 3),
`05-use-cases.md` (§ 3), `07-integration.md`

## Objectif

« ✓ Écrire au calendrier » écrit les rencontres de l'aperçu, après avoir tout
revérifié (R11), et les rend aussitôt ordinaires (R13) : la journée se recharge
avec ses rencontres, chacune supprimable et déplaçable comme aujourd'hui.

## Conception

**Use case** — `use_cases/admin/draw/confirm_draw_use_case.rs` :

```rust
#[tracing::instrument(skip_all, fields(cmd = ?cmd))]
pub async fn execute(cmd: ConfirmDrawCommand, deps: DrawDeps<'_>, event_bus: &EventBus)
    -> Result<ConfirmOutcome, ConfirmDrawError>;
```

1. `load_draw_context` ;
2. `match_day.accept_draw(pairings, cmd.exempted, &rules)` — l'agrégat refuse,
   le use case relaie `Refused(DomainError)` ;
3. un `Pairing` et sa projection par rencontre (`build_new_pairing_projection`) ;
4. `save_pairings` — une transaction sous verrou ; journée remplie entre-temps
   → `PairingsAlreadyExist` ;
5. `PairingCreated` par rencontre **après le commit** (`pairing_created.rs`) ;
6. `ConfirmOutcome` — absentes = inscrites ni appariées ni exemptées.

Les étiquettes de revanche ne viennent jamais du navigateur : la commande ne
porte que les couples.

**Handler** — `schedule_draw_actions::post_draw_confirm`,
`axum_extra::extract::Form<ConfirmDrawForm>` ; chaque `"a:b"` découpé, les deux
ids validés. Succès → `HX-Trigger: scheduleChanged` et `{ success: "5
rencontres écrites · Plague Bringers exemptée · Dwarf Giants absente" }`. Refus
→ 409/422 et `{ error }`, le message de `DomainError` avec les noms résolus.

**`handleScheduleActionResponse`** (`templates/admin/schedule.html`) :
`data.success` devient `window.kreekToast(data.success, 'success')`.

**Route** — `admin_schedule_draw_confirm`.

## Checklist

- [ ] Use case, handler, route ; « Écrire au calendrier » de 600 actif
- [ ] Toast de succès
- [ ] Tests du use case : écriture et émission sur un tirage valide ; chaque
      refus de `accept_draw` relayé sans écriture ni émission ;
      `PairingsAlreadyExist` du dépôt relayé ; **aucune émission si l'écriture
      échoue** ; les absentes déduites
- [ ] `make lint`, `make check-arch`, `make test`
