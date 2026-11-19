> **Carte annulée le 2026-08-25 — fusionnée dans la [398](../ready_to_be_done/398-l-onglet-de-customisation-survit-a-l-enregistrement.md).**
>
> Les deux cartes visaient le même écran et l'on croyait qu'elles partageaient
> une cause. La mesure a montré l'inverse : **elles étaient en tension**. Ce qui
> empêche le défaut de la 327 — le `HX-Refresh` de l'enregistrement — est
> exactement ce que la 326 proposait de supprimer. Les corriger séparément
> aurait fait que l'une défasse l'autre.
>
> Le contenu ci-dessous est conservé tel qu'il était ; la 398 porte le
> diagnostic et la correction.

# `players` — L'onglet de customisation actif ne survit pas à l'enregistrement

**Priorité : haute** — bug de livraison du mode de customisation
**Dépend de :** rien (cartes 302-309 livrées)
**Fichiers :** `src/app/players/io/web/customisation_controller.rs`,
`src/app/players/io/web/templates/player-customisation-widget.html`

## Le problème

Chaque enregistrement d'une demande de customisation rafraîchit la page
entière. L'utilisateur qui travaillait dans l'onglet « Caractéristiques »,
« Prix » ou « SPP » se retrouve sur « Compétences » après chaque validation, et
doit re-cliquer pour continuer. Sur une session de customisation — plusieurs
lignes posées, validées, ajustées — la manipulation se répète à chaque tour.

La cause tient en deux lignes qui, séparément, sont défendables.

Le contrôleur renvoie un rafraîchissement complet après une validation
réussie (`customisation_controller.rs:378`) :

```rust
// La fiche entière change — caractéristiques, prix, SPP, compétences.
// Un swap partiel laisserait la moitié de la page périmée.
Ok(()) => [("HX-Refresh", "true")].into_response(),
```

Et l'onglet actif ne vit que dans l'état Alpine local du panneau
(`player-customisation-widget.html:15`) :

```html
x-data="{ activeTab: 'skills', skillSearch: '' }"
```

Un rechargement complet réinitialise cet état. Le commentaire du contrôleur dit
vrai — la validation change bien les quatre zones de la fiche — donc ce n'est
pas le rafraîchissement qui est fautif, c'est l'onglet qui n'a nulle part où
survivre.

## Ce qu'il faut faire

Deux voies possibles. **La seconde est recommandée.**

1. Remplacer le `HX-Refresh` par des swaps ciblés (`hx-swap-oob`) sur les
   quatre zones qui changent. Fidèle à HTMX, mais coûteux : il faut recenser
   exhaustivement ce que la validation déplace, et toute zone oubliée devient
   périmée en silence — exactement ce que le commentaire actuel cherchait à
   éviter.

2. **Faire survivre l'onglet au rechargement.** Persister `activeTab` (fragment
   d'URL ou `sessionStorage` clé par `player_id`) et le relire à l'`init()` du
   composant Alpine. Le rafraîchissement complet reste, avec sa garantie de
   cohérence ; seul l'onglet est restauré.

Un fragment d'URL (`#custo-spp`) a l'avantage d'être visible, partageable et de
disparaître de lui-même quand on quitte la fiche — pas de résidu à nettoyer,
contrairement au stockage de session.

## Portée — ce que cette carte ne traite pas

`purchase_skill_controller.rs:83` et `increase_stat_controller.rs:67` renvoient
eux aussi un `HX-Refresh`. Ils appartiennent au mode « dépense de SPP », pas au
mode customisation, et leur panneau n'a pas d'onglets. **Les regarder pendant
la correction**, mais ne les modifier que si le même symptôme s'y reproduit —
sinon, une carte à part.

## Checklist

- [ ] L'onglet actif est restauré après l'enregistrement d'une customisation
- [ ] Il l'est aussi après une annulation de ligne et un vidage du panier
- [ ] Rien ne subsiste quand on revient sur la fiche par une autre voie —
      l'onglet par défaut reste « Compétences » à l'arrivée
- [ ] Test e2e : poser une ligne depuis l'onglet « SPP », enregistrer, vérifier
      que « SPP » est toujours l'onglet actif — c'est le seul niveau où le bug
      était observable, aucun test unitaire ne l'aurait vu
- [ ] `make test` passe
- [ ] `make check-arch` passe
