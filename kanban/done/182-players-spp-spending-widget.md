# BC `players` — Widget de dépense de SPP (compétences + caractéristiques)

**Priorité : haute**
**Dépend de :** `177-players-skill-catalog-port.md`, `178-players-purchase-skill-use-case.md`, `179-players-increase-stat-use-case.md`, `181-players-detail-page-widget-slot.md`
**Contexte :** `players/io/web/widgets`

## Objectif

Widget interactif du slot droit quand l'équipe est en phase
`PlayerImprovement` et l'utilisateur autorisé : onglets Compétences
(réutilisation du widget `skill_picker` existant) et Caractéristiques
(nouveau petit widget). Spec complète :
`docs/specs/player-spp-spending/README.md`.

---

## Conception

### Widget `spp_spending_widget` (nouveau)

Template `widgets/spp-spending-panel.html` — tabs Alpine locaux
(Compétences/Caractéristiques), en-tête avec SPP restants
(`player.spp_remaining()`). Onglet Compétences :
```html
<div hx-get="{{ app_routes.references.skill_picker_base() }}?roster_line_id={{ vm.roster_line_id }}&spp_remaining={{ vm.spp_remaining }}&acquired={{ vm.acquired_csv }}"
     hx-trigger="load, skillsUpdated from:body" hx-target="this" hx-swap="innerHTML">
</div>
```
(même patron d'intégration que `finalize-team.html`, cf. recherche phase 1).

Petit relais JS inline écoute `skill-selected` (dispatché par `skill_picker`) :
```js
document.body.addEventListener('skill-selected', (e) => {
  fetch(purchaseUrl, { method: 'POST', headers: {...}, body: JSON.stringify({ skill_id: e.detail.uid, mode: e.detail.mode }) })
    .then(() => window.location.reload());
});
```
(reload complet en succès — pas de rafraîchissement fin, cohérent avec le
patron déjà choisi pour les actions de phase d'équipe.)

### Widget `stat_increase_panel` (nouveau, onglet Caractéristiques)

5 cartes stat (MA/ST/AG/PA/AV), valeur courante (résolue via
`player_stats_service::resolve_stats`, déjà existant), coût du niveau
courant (`player.next_improvement_level()` + `ISkillCatalogPort::cost_for_level().characteristic`),
bouton "Augmenter" en `hx-post` direct vers la route de la carte 179,
`HX-Refresh: true` en succès.

### Permission — re-vérification côté widget

Le handler GET du widget re-vérifie phase + permission indépendamment du
host (défense en profondeur, au cas où l'URL est atteinte directement) —
sinon retourne le widget journal à la place.

---

## Checklist

- [ ] `spp_spending_widget.rs` + template `spp-spending-panel.html`
- [ ] Intégration `skill_picker` existant (hx-get avec `roster_line_id`/`spp_remaining`/`acquired`)
- [ ] Relais JS inline `skill-selected` → POST carte 178 → `HX-Refresh`
- [ ] `stat_increase_widget.rs` (onglet Caractéristiques) + template
- [ ] Re-vérification phase + permission dans le handler GET (fallback vers journal)
- [ ] CSS du panneau (repris de la maquette `app-player-detail.html`, matrice de coût réelle affichée, pas les valeurs plates de la maquette)
