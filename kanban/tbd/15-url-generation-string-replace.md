# Génération d'URLs par string replacement

**Priorité : faible**
**Fichiers :** `src/app/competitions/routes.rs`, `src/app/spaces/routes.rs`, etc.

## Problème

Les URLs sont générées par remplacement de placeholders dans des constantes de chemin :

```rust
pub const COMPETITION_DETAIL: &str = "/app/{space_id}/competitions/{competition_id}/{season_id}";

pub fn competition_detail(&self, sid: &str, cid: &str, season_id: &str) -> String {
    path::COMPETITION_DETAIL
        .replace("{space_id}", sid)
        .replace("{competition_id}", cid)
        .replace("{season_id}", season_id)
}
```

Un `{space_id}` mal orthographié dans une constante compile sans erreur mais produit une URL cassée au runtime. L'ordre des paramètres n'est pas vérifié non plus.

## Action

Plusieurs approches possibles :
- Tests unitaires systématiques sur chaque méthode de `Routes` pour détecter les placeholders non remplacés (`assert!(!url.contains('{'))`)
- Macro procédurale qui vérifie à la compilation que tous les placeholders sont bien remplacés
- Refactoring vers une approche typée (enum de routes avec paramètres)

Le minimum immédiat : ajouter un test par contexte qui vérifie que les URLs générées ne contiennent pas de `{`.
