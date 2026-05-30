# `unwrap_or_default()` silencieux sur les renders Askama

**Priorité : haute**
**Fichiers :** `new_competition.rs`, `competition_detail.rs`, `all_competition.rs`, `news_feed.rs`, et autres

## Problème

Le pattern suivant est répandu dans les handlers :

```rust
let content = tmpl.render().unwrap_or_default();
AppLayout { content, routes: Default::default() }.into_response()
```

Si Askama échoue à rendre le template (données inattendues, bug de template), `unwrap_or_default()` retourne une `String` vide. L'utilisateur reçoit un **200 OK avec une page blanche**, sans log, sans indication d'erreur.

## Action

Traiter l'erreur explicitement :

```rust
let content = tmpl.render().map_err(|e| {
    tracing::error!("render failed: {e}");
    StatusCode::INTERNAL_SERVER_ERROR
})?;
```

Ce point sera naturellement résolu lors de la migration vers `extends` Askama (ticket #16), qui supprime le double rendu.
