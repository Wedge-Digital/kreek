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

## Le piège que cette carte ne nommait pas

Elle dit — à raison — que ne pas relire les administrateurs et le barème les
effacerait. Mais la relecture seule ne suffit pas : `find_base_info` rend
`admin_ids: Vec<String>`, quand `update_base_info` exige `&[CoachId]`. **Il faut
convertir, et la conversion peut échouer.**

Un `filter_map` — le réflexe naturel — écarterait en silence tout administrateur
dont l'identifiant ne se décode pas : exactement la perte que la relecture existe
pour empêcher, obtenue par un autre chemin. Le use case **refuse** donc le
renommage sur une ligne corrompue (`MalformedAdminId`), plutôt que de retirer un
administrateur sans le dire.

C'est aussi ce qui distingue ce use case de `update_draft_competition`, qui
reçoit ses `admin_ids` dans sa commande : le magicien, lui, les édite.

## Un refus par champ, pas un refus unique

La conception parlait d'« un emplacement d'erreur sous le champ Nom », et j'avais
écrit un `Option<String>` unique. Askama a refusé `None::<String>` dans l'appel
de la macro d'upload, ce qui a forcé à relire — et à voir qu'une **URL de logo
invalide se serait affichée sous le nom**, envoyant corriger le mauvais champ.

Le refus porte donc sa cible (`Refus { name, logo }`). Le hasard d'une erreur de
compilation, mais la correction tient sur le fond.

## Ce que les tests affirment

**Les deux relectures sont falsifiées** — c'est le cœur de la carte, et leur
défaut serait invisible à l'écran. Retirer celle des administrateurs fait tomber
deux tests, retirer celle du barème en fait tomber trois.

L'e2e de renommage vérifie que la valeur **survit à un rechargement complet** :
sans cela, un widget qui réafficherait la saisie sans l'enregistrer passerait au
vert.

Deux mutations e2e, chacune sur sa cible. La seconde est la plus vicieuse :
retirer le message du nom déjà pris **tout en laissant le refus opérer** —
l'enregistrement échoue alors sans que rien ne le dise. Un seul test tombe, celui
qui existe pour ça.

## Checklist

- [ ] Le use case et ses tests
- [ ] Les deux handlers, `require_admin_access` sur les deux
- [ ] Le DTO, le VM, le template
- [ ] L'emplacement d'erreur et son style
- [ ] `make lint && make test && make check-arch`
