# `team_creation` — Un corps de requête invalide accuse la compétition

**Priorité : basse**
**Dépend de :** rien
**Fichiers :** `src/app/team_creation/io/web/post_draft_team.rs`

## Objectif

`post_draft_team` traduit **tout** rejet de désérialisation JSON en un seul
message :

```rust
let Json(form) = match body {
    Ok(json) => json,
    Err(_) => return error_response(
        "Veuillez remplir tous les champs obligatoires (compétition, saison)."),
};
```

Le message accuse deux champs qui, dans la plupart des cas, sont parfaitement
renseignés — un champ absent, un type inattendu, un `Content-Type` erroné ou
un corps vide produisent tous la même phrase. Le vrai contrôle de ces deux
champs vit vingt lignes plus bas et a déjà son propre message :

```rust
if form.competition_id.is_empty() || form.season_id.is_empty() {
    return error_response("Sélectionnez une compétition et une saison.");
}
```

Coût constaté : une heure de débogage lors de l'écriture de
`build_and_submit_team_http` (fixtures e2e). La requête envoyait bien
`competition_id` et `season_id`, non vides — elle était simplement encodée en
`application/x-www-form-urlencoded` là où le handler attend du JSON. Le
message désignait la seule piste qui ne pouvait pas être la bonne.

## Pourquoi ça vaut une carte malgré une priorité basse

Le cas est peu atteignable par un utilisateur réel : le formulaire poste du
JSON via l'extension htmx `json-enc`, et une compétition non choisie envoie
une chaîne vide (donc désérialisable), pas un champ absent. C'est donc surtout
un **piège de diagnostic** :

- il masque une erreur d'intégration réelle — un champ renommé côté formulaire
  ou une extension htmx retirée sortiraient sous la forme « compétition et
  saison manquantes », et on chercherait au mauvais endroit ;
- aucune trace n'est émise : `Err(_)` jette le motif du rejet, seul endroit où
  il était disponible.

C'est le seul `JsonRejection` du projet — la correction est locale, il n'y a
pas de pattern à généraliser.

## Conception

Distinguer les deux situations et conserver le motif :

```rust
let Json(form) = match body {
    Ok(json) => json,
    Err(rejet) => {
        // Le motif du rejet n'existe qu'ici : sans trace, un champ renommé
        // côté formulaire devient indiagnosticable.
        tracing::warn!("post_draft_team: corps invalide — {rejet}");
        return error_response(
            "Formulaire invalide — rechargez la page et réessayez.");
    }
};
```

Le message rendu reste actionnable pour l'utilisateur (recharger la page
rétablit un formulaire cohérent) sans désigner de coupable arbitraire. Le
contrôle métier des deux champs, lui, ne bouge pas : il est déjà au bon
endroit avec le bon libellé.

## Checklist

- [ ] Message du rejet JSON dissocié de celui du contrôle compétition/saison
- [ ] Motif du rejet journalisé en `warn`
- [ ] Test : corps non-JSON → message générique, et non « compétition, saison »
- [ ] Test : `competition_id` vide dans un JSON valide → toujours
      « Sélectionnez une compétition et une saison. »
- [ ] `make test` passe
- [ ] `make check-arch` passe
