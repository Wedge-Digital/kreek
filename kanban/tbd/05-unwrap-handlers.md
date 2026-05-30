# `.unwrap()` non protégés dans les handlers

**Priorité : haute**
**Fichier :** `src/app/competitions/io/web/new_competition.rs:247, 272, 355, 369, 417`

## Problème

Plusieurs `.unwrap()` existent dans du code de handler non couvert par des tests :

```rust
// new_competition.rs
.unwrap();   // ligne 247
.unwrap(),   // ligne 272
.unwrap();   // ligne 355
.unwrap(),   // ligne 369
.unwrap(),   // ligne 417
```

Ces appels portent sur des opérations de parsing ou de sérialisation JSON. Un input mal formé ou un état inattendu provoque un **panic → 500 non géré** sans log structuré.

## Action

Remplacer chaque `.unwrap()` par `?` ou par un match explicite avec log d'erreur, en s'assurant que le handler retourne `Result<impl IntoResponse, AppError>` comme préconisé dans le CLAUDE.md.
