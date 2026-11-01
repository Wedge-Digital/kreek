# L'aperçu du tirage

**Épic :** E18 — Appariement des présents · **Ordre :** 6 · **Dépend de :** 616
**Conception :** `docs/specs/appariement-des-presents/calendrier/04-dtos.md` (§ 2),
`05-use-cases.md` (§ 2)
**Maquette :** `assets/rawpages/html/app-competition-admin-schedule-draw.html`, état 3

## Objectif

« Tirer au sort » montre les rencontres tirées parmi les présents, poule par
poule, **sans rien écrire**. L'organisateur peut retirer au sort ou revenir aux
présents, ses décochages conservés.

## Conception

**Use case** — `use_cases/admin/draw/preview_draw_use_case.rs` :

```rust
#[tracing::instrument(skip_all, fields(cmd = ?cmd))]
pub async fn execute(cmd: PreviewDrawCommand, deps: DrawDeps<'_>, rng: &mut impl Rng)
    -> Result<DrawPreview, PreviewDrawError>;
// RoundNotFound | IsRestDay | PairingsAlreadyExist | NotEnoughPresent | Repository
```

`load_draw_context`, `can_receive_draw()`, par poule les présents inscrits →
`DrawInput`, `draw::draw_pools`, puis `DrawPreview` avec noms et historique.
Le générateur est reçu : graine fixe en test, `StdRng::from_os_rng()` dans le
handler.

**Handler** — `schedule_draw_actions::post_draw_preview`,
`axum_extra::extract::Form<PreviewDrawForm>` ; chaque id passe par
`TeamId::try_new`. Succès → le fragment ; refus → 409/422 et `{ error }`.

**VMs** — `DrawPreviewVm`, `PreviewGroupVm`, `PreviewPairingVm`, `BenchTeamVm`,
par `DrawPreviewVm::from_domain(&preview)`. `history_label` : « 1re
rencontre » ou « Revanche · déjà jouée en J1 ». `has_rematch` affiche « Une
revanche n'a pas pu être évitée » — le tirage n'en garde que d'inévitables.

**Gabarit** — `admin/widgets/schedule-draw-preview.html` : rencontres,
exemptée avec son nombre de matchs, absentes ; « Rien n'est écrit au calendrier
tant que vous n'avez pas validé ». Champs cachés `team_ids` (pour « Retirer au
sort », qui renvoie le même formulaire), `pairings` (`home_id:away_id`) et
`exempted` (pour la carte 618). « ← Modifier les présents » rouvre l'étape 616
avec `absent=…`. « ✓ Écrire au calendrier » est présent, inactif jusqu'à la 618.

**Route** — `admin_schedule_draw_preview`.

## Checklist

- [ ] Use case, handler, VMs, gabarit, route ; « Tirer au sort » de 616 actif
- [ ] « Retirer au sort » et « Modifier les présents »
- [ ] `hx-on::after-request` → `handleScheduleActionResponse` sur les refus
- [ ] Tests du use case : rien n'est écrit (doublure de dépôt sans écriture) ;
      deux poules tirées séparément (R3) ; absentes exclues ; aucune poule
      tirable → `NotEnoughPresent` ; graine fixe → résultat reproductible
- [ ] `make lint`, `make check-arch`, `make test`
