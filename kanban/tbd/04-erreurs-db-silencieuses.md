# Erreurs DB avalées silencieusement

**Priorité : haute**
**Fichiers :** `competition_detail.rs:318`, `competition_widget.rs:61`, `new_competition_phase_5.rs:94`

## Problème

Les erreurs de requête base de données sont silencieusement converties en valeur par défaut :

```rust
// competition_detail.rs:318
let season_name = season_info.ok().flatten().map(|s| s.name).unwrap_or_default();

// competition_widget.rs:61
let competitions = state.competitions.competition_repository
    .find_with_seasons(&space_id)
    .await
    .unwrap_or_default();
```

Si la DB est indisponible ou retourne une erreur, la page s'affiche partiellement (champs vides, widget vide) avec un 200 OK. L'erreur n'est ni loguée ni remontée à l'utilisateur.

## Action

Logger l'erreur et retourner un statut approprié :

```rust
let season_name = match season_info {
    Ok(Some(s)) => s.name,
    Ok(None)    => String::new(),
    Err(e)      => {
        tracing::warn!("could not load season info: {e}");
        String::new()
    }
};
```

Pour les cas où la donnée est critique (widget vide = page inutilisable), retourner une erreur HTTP plutôt qu'un contenu dégradé silencieux.
