# `web` — Un panic ne doit pas être l'incident le moins renseigné

**Priorité : moyenne**
**Dépend de :** carte 345 (le span de requête, sans quoi la ligne de panic reste
orpheline)
**Fichiers :** `Cargo.toml`, `src/main.rs`

## Le problème

`tower-http` est compilé avec les features `trace` et `fs` seulement — pas de
`catch-panic`. Un panic dans un handler tue donc la tâche : le client voit une
connexion coupée sans réponse, et le journal reçoit un message de panic brut,
**hors de tout span** — sans identifiant de requête, sans chemin, sans coach.

C'est le pire cas possible : l'incident qui produit le moins d'information est
exactement celui qui en demande le plus. Et comme le client ne reçoit pas de
statut, il n'y a même pas de `500` dans le journal de requêtes pour signaler
qu'il s'est passé quelque chose.

Le projet n'est pas exempt de sources de panic : `unwrap()`, `expect()` et
indexations diverses existent dans le code, y compris sur des chemins de rendu.

## Ce qu'il faut faire

Ajouter la feature `catch-panic` à `tower-http` et poser `CatchPanicLayer`
dans le routeur.

**L'ordre des couches est le point délicat**, et c'est là que la carte se joue :
la couche doit se situer **à l'intérieur** du span de requête, pour que la ligne
de panic hérite du `rid`, du chemin et du coach. Posée à l'extérieur, elle
attrape bien le panic mais le journalise hors contexte — on aurait ajouté un
`500` propre sans rien gagner sur le diagnostic, qui est l'objet de l'épic.

La réponse renvoyée est un `500`. Pas de fragment HTMX élaboré : un panic n'est
pas un cas métier, et rien ne garantit que l'état de l'application permette
encore de rendre quoi que ce soit de sensé.

## Checklist

- [ ] Feature `catch-panic` ajoutée à `tower-http`
- [ ] `CatchPanicLayer` posé **à l'intérieur** du span de requête
- [ ] Un panic provoqué volontairement produit : un `500` côté client, une ligne
      de journal portant le `rid` et le chemin, et le message du panic
- [ ] La ligne de réponse du journal de requêtes montre bien le `500`
- [ ] Test au niveau handler (harnais de la carte 311) sur une route qui panique
- [ ] `make test` et `make check-arch` passent
