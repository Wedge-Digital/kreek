# BC `competitions` — Page hôte admin compétition (layout commun)

**Priorité : haute**
**Dépend de :** rien
**Contexte :** BC `competitions` — administration de compétition
**Spec :** `docs/specs/competition-admin/dashboard/03-back.md`, `04-dtos.md`, `07-integration.md`

## Objectif

Créer la page hôte de l'administration de compétition : banner, tabs (6 onglets), zone de contenu `#admin-content`. Ce layout est partagé par tous les onglets admin. Le guard admin (admin espace OU admin compétition) est vérifié ici.

---

## Fichiers à créer

| Fichier | Rôle |
|---|---|
| `src/app/competitions/io/web/admin/mod.rs` | Module admin |
| `src/app/competitions/io/web/admin/admin_page.rs` | Handler page hôte + `AdminPageTemplate` |
| `src/app/competitions/io/web/templates/admin-page.html` | Template (extends `app-layout.html`) : banner + tabs + `#admin-content` |
| `assets/static/css/pages/competition-admin.css` | Styles partagés : banner, tabs, panel, boutons |

## Fichiers à modifier

| Fichier | Modification |
|---|---|
| `src/app/competitions/io/web/mod.rs` | Ajouter `pub mod admin;` |
| `src/app/competitions/routes.rs` | Ajouter route `/admin` |
| `src/app/competitions/router.rs` | Câbler le handler |

## Détails

### `AdminPageTemplate`

```rust
#[derive(Template)]
#[template(path = "admin-page.html")]
pub struct AdminPageTemplate {
    pub app_routes: AppRoutes,
    pub space_id: String,
    pub competition_id: String,
    pub season_id: String,
    pub competition_name: String,
    pub season_name: String,
    pub admin_count: usize,
    pub active_tab: String,
    pub content: String,
}
```

### Guard admin

Vérification dans le handler :
- Charger la compétition
- Vérifier que l'utilisateur est admin de l'espace OU que son coach_id est dans `competition.admin_ids`
- Retourner 403 sinon

### Contenu inline

Au premier chargement, `content` contient le HTML du fragment de l'onglet par défaut (dashboard). Pour cette carte, un placeholder statique suffit : `"<p>Tableau de bord — à venir</p>"`.

### Tabs

Chaque tab fait un `hx-get` vers le fragment correspondant avec `hx-target="#admin-content"` et `hx-push-url="true"`.

---

## Checklist

- [ ] Créer `src/app/competitions/io/web/admin/mod.rs`
- [ ] Créer `admin_page.rs` avec `AdminPageTemplate` et handler
- [ ] Implémenter le guard admin (admin espace OU admin compétition)
- [ ] Créer `admin-page.html` : banner + tabs + `#admin-content`
- [ ] Créer `competition-admin.css` (styles partagés)
- [ ] Ajouter la route dans `routes.rs` et `router.rs`
- [ ] Vérifier que la page se charge avec le placeholder
