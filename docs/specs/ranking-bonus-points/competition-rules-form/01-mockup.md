# Phase 1 — Maquette (competition-rules-form)

Extension de la section « Bonus de classement » du formulaire admin phase-2
(`new-competition-phase-2.html`, lignes 64-87). **Aucune nouvelle CSS** :
réutilisation stricte des classes existantes (`.bonus-row`, `.bonus-check`,
`.bonus-label`, `.bonus-inline`, `.bonus-num`, `.label-tiny`).

## Markup validé

```html
<div class="rules-section">
  <div class="rules-section-title">Bonus de classement</div>

  <!-- Offensif (existant, wording précisé "marqués") -->
  <div class="bonus-row">
    <input type="checkbox" id="off_activated" class="bonus-check" checked>
    <span class="bonus-label">Bonus offensif : +</span>
    <div class="bonus-inline">
      <input type="number" id="off_points" class="bonus-num" value="1" min="0">
      <span class="label-tiny">pt si ≥</span>
      <input type="number" id="off_diff_td" class="bonus-num" value="3" min="1">
      <span class="label-tiny">TDs marqués</span>
    </div>
  </div>

  <!-- Défensif : seuil désormais configurable -->
  <div class="bonus-row">
    <input type="checkbox" id="def_activated" class="bonus-check" checked>
    <span class="bonus-label">Bonus défensif : +</span>
    <div class="bonus-inline">
      <input type="number" id="def_points" class="bonus-num" value="1" min="0">
      <span class="label-tiny">pt si ≤</span>
      <input type="number" id="def_max_td" class="bonus-num" value="1" min="0">
      <span class="label-tiny">TD encaissés</span>
    </div>
  </div>

  <!-- Agressif : nouveau -->
  <div class="bonus-row mb-0">
    <input type="checkbox" id="agg_activated" class="bonus-check">
    <span class="bonus-label">Bonus agressif : +</span>
    <div class="bonus-inline">
      <input type="number" id="agg_points" class="bonus-num" value="1" min="0">
      <span class="label-tiny">pt si &gt;</span>
      <input type="number" id="agg_min_cas" class="bonus-num" value="2" min="0">
      <span class="label-tiny">sorties infligées</span>
    </div>
  </div>
</div>
```

## Décisions de design

- **Agressif décoché par défaut** (`activated=false`) — bonus nouveau/optionnel ;
  offensif et défensif restent cochés (défaut actuel inchangé).
- **Seuil défensif** = input `def_max_td` (défaut `1`, rétro-compat affichage actuel).
- **Comparateur agressif** `>` strict, défaut Y=2 sorties.
- Réutilisation des classes CSS existantes → zéro CSS ajoutée.

## Règles métier confirmées à cette étape

Voir le tableau de règles partagé dans le `README.md`. À cette étape (saisie) :
chaque bonus a un flag `activated` propre ; le calcul (unité `post-match-bonus-calc`)
ignore un bonus désactivé.