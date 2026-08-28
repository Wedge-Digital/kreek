# Donner un portrait à un joueur

**Priorité : basse** — du confort, aucun défaut réparé
**Périmètre : le BC `players`, de la base à l'écran**
**Dépend de :** rien. Voisine de la **carte 461** (logo d'équipe), mais elle ne
partage aucun code.

## Objectif

Cliquer sur l'avatar d'un joueur ouvre le widget Cloudinary, comme sur la fiche
d'équipe. Le portrait remplace le numéro, qui passe en pastille.

## Ce que la 461 avait et que celle-ci n'a pas

Sur l'équipe, la moitié du travail dormait déjà : `logo_url` en base,
`LogoChanged` dans l'agrégat. Ici **rien n'existe**.

| | Équipe (carte 461) | Joueur |
|---|---|---|
| Colonne en base | `team_proj.logo_url` ✓ | **manque** |
| Champ d'agrégat | `Team.logo_url` ✓ | **manque** |
| Événement | `LogoChanged` ✓ | **manque** |
| Émetteur | manquant | manquant |
| Projection | manquante | manquante |

Ce que la fiche affiche aujourd'hui en guise d'avatar, c'est **le numéro de
maillot** sur un dégradé rouge (`player-page.css:22`), ou un tiret s'il n'en a
pas.

**Cette carte traverse donc toute la pile** — migration, domaine, événement,
projection, use case, route, écran — là où la 461 n'en réveillait qu'une partie.

## Conception

### 1. La migration

```sql
ALTER TABLE players_proj ADD COLUMN portrait_url TEXT;
```

**Nullable, sans défaut.** Un joueur sans portrait est le cas normal, pas une
exception à rattraper : les 49 000 lignes existantes gardent `NULL` et
continuent d'afficher leur numéro.

### 2. Le domaine

```rust
// players/domain/events.rs
PlayerPortraitChanged {
    player_id: PlayerId,
    team_id: TeamId,
    portrait_url: Option<CloudinaryImage>,
}
```

**`Option`, et le `None` est signifiant** — il efface le portrait. C'est
exactement ce que fait `PlayerRenamed`, dont le commentaire dit :

> Le `Option::None` est signifiant — il efface la valeur.

**Le patron à suivre est `PlayerRenamed`**, pas `LogoChanged` de la carte 461 :
il porte déjà le `team_id` à côté du `player_id`, ce que la projection exige.

```rust
// players/domain/player.rs
pub fn change_portrait(&self, portrait: Option<CloudinaryImage>)
    -> Result<PlayerDomainEvent, DomainError>
```

**Aucune garde de phase.** Un portrait n'a de conséquence ni sur la valeur, ni
sur les SPP, ni sur la disponibilité. Le restreindre empêcherait un coach de
corriger une image pendant un match.

### 3. La projection

`player_repository.rs` traite déjà `PlayerJerseyChanged` (lignes 68 et 460 — un
bras pour l'append, un pour le rejeu). `PlayerPortraitChanged` en a besoin **des
deux**.

**C'est le geste sans lequel rien ne se verra** : l'agrégat changerait, la
projection non, et la fiche afficherait l'ancien portrait. C'est exactement le
piège identifié sur la carte 461, et il se répète ici.

### 4. Le use case et l'accès

```rust
// players/use_cases/change_player_portrait_use_case.rs
#[tracing::instrument(skip_all, fields(cmd = ?cmd))]
pub async fn execute(cmd, repo, …) -> Result<(), ChangePortraitError>
```

**Le contrôle d'accès est `can_spend_spp`, pas `can_customise`.** La fiche
joueur en porte deux, et son propre commentaire prévient qu'on les confond :

> `can_customise` : admin d'espace ou admin de la compétition, et personne
> d'autre. Le coach de l'équipe en est exclu — un coach qui s'ajouterait des
> compétences gratuitement ne serait pas la même fonctionnalité.
> C'est `can_spend_spp` qui, lui, est explicitement « étendu au coach ».

Un portrait relève du second : **c'est le coach qui doit pouvoir le poser**, pas
seulement un commissaire. Prendre `can_customise` par mimétisme le lui
interdirait.

### 5. La route

```
POST /app/{space_id}/players/{player_id}/portrait
```

`space_scope` couvre `{player_id}` — `players` déclare son résolveur
(`infrastructure/players/space_ownership.rs`).

Au retour, `HX-Refresh: true` : le portrait apparaît aussi dans le tableau de
l'effectif, et un swap partiel en laisserait un périmé.

### 6. L'écran

Le portrait **remplace** le numéro dans `.player-avatar`, et le numéro passe en
**pastille** sur le coin de l'image.

Le numéro reste l'identifiant du joueur sur le terrain — on ne peut pas le
perdre — mais un visage vaut mieux qu'un chiffre quand il existe.

**Un joueur sans portrait garde exactement l'écran d'aujourd'hui** : le numéro
sur son dégradé rouge, et pas de pastille.

L'avatar devient cliquable **sous condition d'accès**, et ouvre la macro
`cmp::cloudinary_upload` — celle qu'emploient `draft-team.html`,
`new-competition-phase-1.html` et `new-article.html`. Un survol doit signaler
que l'image réagit.

Le CSS va dans `pages/player-page.css`, déjà au bundle. **Aucune feuille neuve.**

## Ce que la carte ne fait pas

- **Aucun portrait dans le tableau de l'effectif** : la vignette y serait
  minuscule, et la carte 460 vient d'y ajouter une ligne. À décider plus tard.
- **Aucun portrait par défaut** — pas d'avatar généré, pas d'image de
  remplacement. Le numéro sur son dégradé reste la solution du joueur sans
  photo.
- **Aucune modération** : ce que Cloudinary accepte, l'application l'affiche.

## Tests

| Test | Ce qu'il prouve |
|---|---|
| `change_portrait_emet_l_evenement` | le domaine |
| `un_portrait_none_efface_l_existant` | le `None` signifiant |
| `le_projecteur_ecrit_portrait_url` | **le geste qui manquerait** |
| `le_rejeu_restitue_le_portrait` | le second bras de projection |
| `une_url_invalide_est_refusee` | `CloudinaryImage` |
| `le_coach_de_l_equipe_est_autorise` | **`can_spend_spp`, pas `can_customise`** |
| **E2E** : cliquer l'avatar ouvre le widget, la fiche montre le portrait | bout en bout |

Le sixième est celui qui attrape l'erreur la plus probable : prendre le mauvais
contrôle d'accès par mimétisme avec la customisation.

## Checklist

- [ ] La migration, colonne nullable
- [ ] `PlayerPortraitChanged` sur le modèle de `PlayerRenamed`
- [ ] `change_portrait`, sans garde de phase
- [ ] **Les deux bras de projection** — append et rejeu
- [ ] Le use case, instrumenté, avec `can_spend_spp`
- [ ] La route, le handler, `HX-Refresh`
- [ ] L'avatar cliquable, la pastille du numéro
- [ ] Les six tests unitaires et le test e2e
- [ ] `make lint && make test && make check-arch`
