# Changer le logo d'une équipe

**Priorité : moyenne** — un logo choisi à la création ne se corrige nulle part
**Périmètre : la fiche d'équipe, BC `teams`**
**Dépend de :** rien

## Objectif

Cliquer sur le logo de l'équipe ouvre le widget Cloudinary, comme partout
ailleurs dans l'application. Le nouveau logo remplace l'ancien.

## Le constat — la moitié du travail est déjà là, et dort

`TeamDomainEvent::LogoChanged { logo_url }` **existe** (`team.rs:217`), et
l'agrégat sait l'appliquer :

```rust
TeamDomainEvent::LogoChanged { logo_url } => {
    self.logo_url = Some(logo_url.clone());
}
```

**Mais personne ne l'émet, et rien ne le projette.**

```bash
grep -rn "LogoChanged" src/    # 4 occurrences, toutes dans domain/team.rs
```

C'est le même cas que `CostlyMistakesApplied` avant l'épic E13 : l'aval a été
écrit avant son producteur, et il attend depuis.

**Le projecteur ne le traite pas non plus** : `team_repository.rs` écrit
`logo_url` sur `TeamCreated` mais n'a aucun bras pour `LogoChanged`. Un
événement appendu sans ce bras changerait l'agrégat et **pas** la projection —
donc la fiche continuerait d'afficher l'ancien logo jusqu'au prochain rejeu.

## Conception

### 1. La méthode d'agrégat

```rust
pub fn change_logo(&self, logo: CloudinaryImage) -> Result<TeamDomainEvent, DomainError> {
    Ok(TeamDomainEvent::LogoChanged { logo_url: logo.into_inner() })
}
```

**Aucune garde de phase.** Changer un logo n'est pas un geste de jeu : il n'a de
conséquence ni sur la valeur d'équipe, ni sur l'effectif, ni sur la trésorerie.
Le restreindre à une phase serait une contrainte sans motif — et empêcherait un
coach de corriger une image pendant un match.

`CloudinaryImage` est le value object employé par `competitions` et
`team_creation` pour le même besoin ; il valide l'URL.

### 2. Le projecteur gagne son bras

```rust
TeamDomainEvent::LogoChanged { logo_url } => {
    UPDATE team_proj SET logo_url = $2, updated_at = now() WHERE team_id = $1
}
```

**C'est le geste sans lequel rien ne se verra.** L'agrégat changerait, la
projection non, et la fiche afficherait l'ancien logo — un défaut qui se
diagnostique mal parce que l'événement, lui, est bien écrit.

### 3. Le use case

```rust
// teams/use_cases/change_team_logo_use_case.rs
#[tracing::instrument(skip_all, fields(cmd = ?cmd))]
pub async fn execute(
    cmd: ChangeTeamLogoCommand,     // team_id, logo, changed_by
    repo: &dyn ITeamRepository,
    access: &dyn ITeamAccessPort,
) -> Result<(), ChangeTeamLogoError>
```

**Le contrôle d'accès est celui de l'effectif** :
`roster_edit_access_service::peut_modifier_effectif` — propriétaire de l'équipe,
admin d'espace ou admin de compétition (carte 389).

Le réutiliser plutôt que d'en écrire un second : ce sont les mêmes personnes, et
deux services de droits pour une même fiche divergeraient.

### 4. La route et le handler

```
POST /app/{space_id}/teams/{team_id}/logo
```

```rust
#[derive(Deserialize)]
pub struct ChangeLogoForm { pub logo_url: String }
```

`space_scope` couvre `{team_id}`. Le handler construit le `CloudinaryImage` —
une URL invalide est un `422` avant le use case.

Au retour : `HX-Refresh: true`, ou le re-rendu de l'en-tête. **Le rechargement
complet est plus sûr** : le logo apparaît aussi dans le bandeau et le menu, et
un swap partiel en laisserait un périmé.

### 5. L'écran

Le logo de `teams-team-detail.html` devient cliquable **si `peut_editer`**, et
ouvre la macro `cmp::cloudinary_upload` — celle qu'emploient déjà
`draft-team.html`, `new-competition-phase-1.html` et `new-article.html`.

**La macro embarque son propre chargeur Cloudinary**, avec un commentaire qui
explique pourquoi il n'est pas dans le layout : il tire Google Tag Manager, GA4
et Rollbar, et il n'y a aucune raison que ces trois-là suivent un coach sur
toutes les pages.

Un survol doit dire que c'est cliquable — le logo est une image, rien n'indique
qu'elle réagit.

## Ce que la carte ne fait pas

- **Aucune suppression de logo** : on le remplace, on ne le retire pas. Une
  équipe sans logo affiche ses initiales, ce qui reste le cas si l'URL est vide.
- **Aucun historique** : l'event store garde les `LogoChanged` successifs, mais
  aucun écran ne les montre.
- **Aucun redimensionnement** : Cloudinary s'en charge.

## Tests

| Test | Ce qu'il prouve |
|---|---|
| `change_logo_emet_l_evenement` | l'agrégat |
| `le_projecteur_met_a_jour_logo_url` | **le geste qui manquait** |
| `une_url_invalide_est_refusee` | le value object |
| `un_non_autorise_est_refuse` | le contrôle d'accès |
| **E2E** : cliquer le logo ouvre le widget, et la fiche montre le nouveau | bout en bout |

Le second est celui qui compte : sans lui, un événement bien écrit laisserait la
fiche inchangée, et le défaut se chercherait du côté de Cloudinary.

## Checklist

- [ ] `change_logo` sur l'agrégat, sans garde de phase
- [ ] **Le bras de projection pour `LogoChanged`**
- [ ] Le use case, instrumenté, réutilisant `peut_modifier_effectif`
- [ ] La route, le handler, `HX-Refresh`
- [ ] Le logo cliquable sous condition, avec la macro existante
- [ ] Les quatre tests unitaires et le test e2e
- [ ] `make lint && make test && make check-arch`
