# `cloudinary_transform()` privée et non réutilisable

**Priorité : faible**
**Fichier :** `src/app/competitions/io/web/competition_detail.rs:10`

## Problème

Une fonction utilitaire pour transformer les URLs Cloudinary est définie en `fn` privée dans un seul fichier handler :

```rust
fn cloudinary_transform(url: &str, transform: &str) -> String {
    const MARKER: &str = "/upload/";
    if let Some(pos) = url.find(MARKER) {
        let (before, after) = url.split_at(pos + MARKER.len());
        format!("{}{}/{}", before, transform, after)
    } else {
        url.to_string()
    }
}
```

Dès qu'un autre handler a besoin de transformer une URL Cloudinary (avatar de coach, logo de space), cette fonction sera copiée.

## Action

Déplacer dans `src/app/shared_kernel/` ou dans un module dédié `src/lib/cloudinary.rs`. Ajouter un test unitaire sur le cas où l'URL ne contient pas `/upload/`.
