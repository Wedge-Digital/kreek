# BC match_report — Controller + HTML inducements (extension mercenaires)

**Priorité : haute**
**Dépend de :** 126, 127
**Contexte :** `docs/specs/match-report/step2-mercenaires/02-front.md`, `03-back.md`, `04-dtos.md`

## Objectif

Étendre le contrôleur et les templates pour intégrer les mercenaires dans le formulaire POST et la page inducements (4 tabs, panier étendu, tab bar migration).

## Conception

### 1. InducementsForm — inducements_controller.rs

Ajouter le champ `mercenaries` :

```rust
#[derive(Deserialize)]
pub struct InducementsForm {
    #[serde(default)]
    pub selection:   String,
    #[serde(default)]
    pub mercenaries: String,   // NOUVEAU — JSON "[{position_uid, level}]"
}
```

### 2. Handler POST — parsing mercenaires

Dans le handler POST, après le parsing `selection` existant, ajouter :

```rust
let mercenary_purchases = parse_mercenaries(&form.mercenaries)?;
// Construit Vec<MercenaryPurchaseCmd> depuis le JSON
// Erreur 400 si position_id invalide (PositionId::try_new) ou level inconnu
```

Fonction `parse_mercenaries` (< 20 lignes) : désérialise JSON, mappe en `MercenaryPurchaseCmd`.

### 3. InducementsTemplate — ajout mercenary_selector_url

```rust
pub struct InducementsTemplate {
    // ... champs existants ...
    pub mercenary_selector_url: String,   // NOUVEAU
}
```

Dans `build_vm`, construire :
```rust
mercenary_selector_url: routes.match_report.mercenary_selector(&space_id, &mr_id, &team_id),
```

### 4. Template inducements.html

Modifications selon `02-front.md` :

**Tab bar (dans la page hôte)** : remplacer les 3 tabs actuels par 4 tabs (Communs / Spéciaux / Stars / Mercenaires) gérés par Alpine `activeSection` de la page.

**Zone Mercenaires** : conteneur chargé en lazy HTMX :
```html
<div hx-get="{{ mercenary_selector_url }}"
     hx-trigger="mercenairesActivated from:body once"
     hx-target="this">
</div>
```

**Formulaire** : ajouter `<input type="hidden" name="mercenaries" x-bind:value="JSON.stringify(mercenaryCart)">` synchronisé par événement `mercenarySelectionChanged`.

**Panier (cart footer)** : étendre pour afficher les mercenaires avec bouton ✕ par mercenaire.

### 5. Template inducement-selector.html — BC références

**Retirer** le bloc `<div class="mr-tabs">` (tab bar).

**Ajouter** un listener Alpine sur `switchInducementTab` :
```html
<div x-data="inducementSelector()" @switch-inducement-tab.window="activeTab = $event.detail.tab">
```

Ou dans le JS Alpine existant, dans `init()` :
```js
document.body.addEventListener('switchInducementTab', (e) => {
    this.activeTab = e.detail.tab;
});
```

## Checklist

- [ ] `InducementsForm.mercenaries` ajouté
- [ ] `parse_mercenaries` implémentée (validation `PositionId::try_new` + `MercenaryLevel::try_from_str`)
- [ ] Handler POST passe `mercenary_purchases` dans la commande
- [ ] `InducementsTemplate.mercenary_selector_url` ajouté + construit dans `build_vm`
- [ ] `inducements.html` : 4 tabs, zone lazy merco, champ hidden, cart étendu
- [ ] `inducement-selector.html` (BC refs) : tab bar retirée, event listener ajouté
- [ ] Comportement Alpine `switchInducementTab` fonctionne (tab bar page ↔ widget références)
- [ ] Comportement `mercenarySelectionChanged` → panier mis à jour
- [ ] `cargo build` passe
- [ ] Vérification manuelle flow complet : sélection mercenaire → ajout panier → soumission → step 3
