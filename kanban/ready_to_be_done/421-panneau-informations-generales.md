# Panneau « Informations générales »

**Épic :** E14 · **Ordre :** 3 · **Dépend de :** 420
**Conception :** `docs/specs/modifier-une-competition/onglet-parametres/`
(`04-dtos.md`, `05-use-cases.md`)

## Objectif

Renommer une compétition, renommer sa saison, changer son logo. Le panneau le
plus simple, et celui qui pose la forme des quatre autres.

## Conception

### Le use case

```rust
// use_cases/settings/update_general_settings_use_case.rs
pub struct UpdateGeneralSettingsCommand {
    pub competition_id: CompetitionId,
    pub space_id: SpaceId,
    pub season_id: SeasonId,
    pub name: CompetitionName,
    pub season_name: SeasonName,
    pub logo: CloudinaryImage,
}
```

1. `find_base_info(&competition_id)` → `CompetitionNotFound`
2. si le nom change, `name_exists_in_space` → `NameAlreadyTaken`
3. `update_base_info(&competition_id, &name, &logo, &current.admin_ids)`
4. `find_rules(&season_id)` → `SeasonNotFound`
5. `save_rules(&season_id, &cmd.season_name, &current_rules)`

**Deux écritures, un seul use case.** Le nom de compétition vit dans
`competitions`, celui de saison dans `competition_seasons`, mais l'intention est
une — et le `CLAUDE.md` interdit au handler d'appeler deux use cases.

**Les deux relectures sont le cœur de la carte.** `update_base_info` porte
`admin_ids` et `save_rules` porte les `rules` : ne pas les relire viderait les
administrateurs et tout le barème. Le panneau n'édite ni l'un ni l'autre.

Pas de transaction commune : deux libellés qu'aucun invariant ne lie, et l'écran
renvoie l'état réel au retour.

Instrumenté (`#[tracing::instrument(skip_all, fields(cmd = ?cmd))]`).

### Le handler

```rust
GET  …/settings/general   → get_settings_general
POST …/settings/general   → post_settings_general   (axum::Form)
```

`require_admin_access` en première ligne des deux.

```rust
#[derive(Deserialize)]
pub struct GeneralSettingsForm {
    pub name: String,
    pub season_name: String,
    pub logo_url: String,
}
```

Le handler construit les value objects par leurs smart constructors — un échec
est un `422` avec le widget re-rendu.

### Le VM et le template

```rust
pub struct GeneralVm {
    pub name: String,
    pub season_name: String,
    pub logo_url: String,
    pub admins: Vec<AdminRowVm>,     // affichage seul
}
pub struct AdminRowVm { pub coach_name: String, pub is_owner: bool }
```

`GeneralVm::from_domain(&CompetitionBaseInfo, &SeasonBaseInfo)` — purement
domaine, constructeur co-localisé.

`templates/admin/widgets/settings-general.html`, racine
`hx-disinherit="*"`, `hx-swap="outerHTML"` sur elle-même au retour du POST.

Le logo reprend la macro `cmp::cloudinary_upload` de
`new-competition-phase-1.html:78`.

**L'emplacement d'erreur sous le champ Nom** — `.form-row-error`, réservé en
permanence dans le flux pour que le formulaire ne saute pas au moment où
l'utilisateur lit le reproche. Alimenté par `SettingsGeneralWidget.error`.

## Tests

- Unitaires : les deux relectures (administrateurs et règles préservés), le
  refus de nom déjà pris, le nom inchangé qui ne déclenche pas le contrôle
  d'unicité.
- E2E : renommer la compétition, et voir l'erreur sous le champ sur un doublon.

## Checklist

- [ ] Le use case et ses tests
- [ ] Les deux handlers, `require_admin_access` sur les deux
- [ ] Le DTO, le VM, le template
- [ ] L'emplacement d'erreur et son style
- [ ] `make lint && make test && make check-arch`
