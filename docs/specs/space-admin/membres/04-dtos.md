# Page hôte + onglet Membres — Contrats de données

**Entrée :** `03-back.md` validé.

Chaque type porte ici son **émetteur** et son **consommateur** : c'est ce que la
phase impose, et c'est ce qui permet de voir d'un coup d'œil qu'un DTO de port
ne remonte jamais jusqu'à un template.

## DTOs d'entrée

### Paramètres de chemin

Les deux actions et les deux widgets sont paramétrés par `{space_id}` ;
les actions le sont aussi par `{coach_id}`.

`space_id` **n'est pas extrait par le contrôleur** : `SpacePermissions` le fait
déjà, le valide en `SpaceId`, et rend 403 si l'appelant n'est pas membre. Le
contrôleur lit `perms.space_id` — extraire une seconde fois donnerait deux
sources de vérité pour la même valeur.

`coach_id` est validé par le contrôleur via `CoachId::try_new()`, 400 sinon.

| DTO | Émetteur | Consommateur |
|---|---|---|
| `Path<String>` (coach_id) | axum | contrôleur, qui en fait un `CoachId` |
| `SpacePermissions` | extracteur `spaces` | contrôleur, pour `space_id` et `is_admin()` |

### Corps de formulaire

```rust
#[derive(Deserialize)]
pub struct ChangeRoleForm {
    /// "SpaceAdmin" | "SpaceUser" — la représentation de `SpaceProfile::as_str()`
    pub profile: String,
}
```

Primitive assumée : c'est la frontière HTTP, où rien n'est encore validé. Le
contrôleur la convertit par `SpaceProfile::try_from(&str)`, qui existe déjà et
rend une erreur sur toute autre valeur.

Le retrait n'a **pas de corps** : la cible est dans le chemin, l'acteur dans la
session.

| DTO | Émetteur | Consommateur |
|---|---|---|
| `ChangeRoleForm` | `<kreek-select>` de la ligne, en POST | `change_member_role_controller` |

## Commandes applicatives

Aucune primitive nue — règle CQRS du `CLAUDE.md`.

```rust
pub struct ChangeMemberRoleCommand {
    pub space_id:       SpaceId,
    pub actor:          CoachId,
    pub target:         CoachId,
    pub nouveau_profil: SpaceProfile,
}

pub struct RemoveMemberCommand {
    pub space_id: SpaceId,
    pub actor:    CoachId,
    pub target:   CoachId,
}
```

**L'acteur vient de `AuthSession`, jamais du formulaire.** Deux règles portent
sur lui — on ne modifie pas son propre rôle, on ne se retire pas soi-même — et
une identité qui transite par le client est une identité qu'on peut réécrire.

Les deux commandes sont instrumentées par le `#[tracing::instrument(skip_all,
fields(cmd = ?cmd))]` de leur use case. Aucun champ sensible ici, donc pas de
`Secret<T>` à prévoir.

| DTO | Émetteur | Consommateur |
|---|---|---|
| `ChangeMemberRoleCommand` | `change_member_role_controller` | `change_member_role_use_case` |
| `RemoveMemberCommand` | `remove_member_controller` | `remove_member_use_case` |

## DTO de lecture du dépôt

```rust
pub struct SpaceMemberRow {
    pub coach_id:   String,
    pub coach_name: String,
    pub email:      String,
    pub icon:       Option<String>,
    pub profile:    String,   // "SpaceAdmin" | "SpaceUser"
}
```

Primitives acceptées : c'est un DTO de lecture rendu par une méthode `list_*`
du port, et il ne porte aucun invariant à protéger.

**Une seule requête pour les deux widgets.** Le widget stats compte les membres
et les administrateurs sur cette même liste. Un `SELECT count(*)` séparé
donnerait deux lectures pour une donnée que la première contient déjà.

| DTO | Émetteur | Consommateur |
|---|---|---|
| `SpaceMemberRow` | `ISpaceRepository::list_members_with_profile` | `builders.rs` des deux widgets, **jamais un template** |

## View models

### La ligne de membre

```rust
pub struct MemberRowVm {
    pub coach_id:     String,
    pub name:         String,
    pub email:        String,
    pub initials:     String,
    pub is_self:      bool,
    pub is_admin:     bool,
    pub role_locked:  bool,
    pub removable:    bool,
    pub reset_action: String,
}
```

`initials` réutilise `crate::common::initials::initials`, déjà employé par
`coach_search_results`.

`reset_action` est l'URL rendue par `host_layout.password_reset_action(&name)`.
Le VM la porte **déjà résolue** : le template ne doit pas avoir à savoir d'où
elle vient.

### `role_locked` et `removable` — ce qui fait foi

```
role_locked = is_self  ||  (is_admin && nombre_d_admins == 1)
removable   = !is_self && !(is_admin && nombre_d_admins == 1)
```

**Ces deux booléens sont une politesse, pas une garde.** Ils grisent le
`<kreek-select>` et retirent le bouton, pour que l'interface ne propose pas ce
qu'elle refusera. La règle qui fait foi vit dans `Space::change_member_role` et
`Space::remove_member` : un client qui contourne le grisage se fait refuser par
le domaine, avec `SpaceMembershipError::DernierAdministrateur` ou
`ActeurEstLaCible`.

C'est la répartition que le `CLAUDE.md` impose — « le front grise, le domaine
refuse » — et elle se vérifie en test : un POST direct sur le dernier
administrateur doit échouer, sans passer par l'interface.

### Les templates

```rust
#[derive(Template)]
#[template(path = "widgets/space-admin-members.html")]
pub struct SpaceAdminMembersTemplate {
    pub routes:   Routes,
    pub space_id: String,
    pub members:  Vec<MemberRowVm>,
}

#[derive(Template)]
#[template(path = "widgets/space-admin-stats.html")]
pub struct SpaceAdminStatsTemplate {
    pub membres:                usize,
    pub administrateurs:        usize,
    pub invitations_en_attente: usize,   // 0 jusqu'à l'onglet Invitations
}

#[derive(Template)]
#[template(path = "space-admin.html")]
pub struct SpaceAdminPageTemplate {
    pub routes:         Routes,
    pub space_id:       String,
    pub space_name:     String,
    pub content_target: String,
}
```

`routes` est le `Routes` **de `spaces`**, pas `AppRoutes` : le BC est
extractible.

| VM | Émetteur | Consommateur |
|---|---|---|
| `MemberRowVm` | `builders.rs` du widget membres | `space-admin-members.html` |
| `SpaceAdminMembersTemplate` | `space_admin_members_widget` | Askama |
| `SpaceAdminStatsTemplate` | `space_admin_stats_widget` | Askama |
| `SpaceAdminPageTemplate` | `space_admin_controller` | Askama, puis `host_layout.wrap_page()` |

### `builders.rs` et non `from_domain()`

`MemberRowVm` se construit à partir de `SpaceMemberRow` — un DTO de port — et
d'une donnée qui n'est ni dans l'un ni dans l'autre : l'URL de réinitialisation,
qui vient du `host_layout`. Le `CLAUDE.md` tranche : constructeur `from_domain()`
co-localisé pour les VMs purs domaine, fonction dans `builders.rs` dès qu'un DTO
de port entre en jeu. C'est le second cas.

## Le badge de visibilité — absent de cette livraison

La bannière de la maquette porte un badge 🔒 Privé / 🌐 Public. **La visibilité
n'existe pas** : ni colonne, ni valeur, ni écran pour la régler. Elle arrive
avec l'onglet Paramètres.

`SpaceAdminPageTemplate` ne porte donc **pas** de champ de visibilité, et le
badge est absent du gabarit. L'onglet Paramètres l'ajoutera aux deux.

Les deux autres voies ont été écartées. Avancer la colonne dès maintenant ferait
entrer une conception de l'onglet Paramètres dans la carte de la page hôte, pour
un ornement. Sortir l'onglet Paramètres en premier inverserait un ordre qui a
ses raisons — les trois autres onglets s'appuient sur les fondations de
celui-ci.

## Ce qui n'existe pas ici

**Aucun DTO de port inter-BC.** Le sens de la propagation est sortant, et
`spaces__user_cache` contient déjà ce qu'il faut. Pas de `ports.rs` à créer,
donc pas de domain service de transformation non plus.

## Questions ouvertes pour la phase 5

- Le use case de changement de rôle doit-il rendre la **ligne mise à jour**, ou
  seulement l'événement ? Le contrôleur a besoin de re-rendre la ligne, et il
  lui faut alors le nombre d'administrateurs postérieur au changement pour
  recalculer `role_locked`. Relire la liste après coup est le plus simple ; le
  faire rendre par le use case évite une seconde lecture.
