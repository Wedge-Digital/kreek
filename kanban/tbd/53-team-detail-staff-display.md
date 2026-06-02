# BC `teams` — Affichage du staff dans la fiche d'équipe

**Priorité : moyenne**
**Dépend de :** `51-team-created-staff-transport.md`, `52-team-staff-tracking.md`, `34-team-detail-wired.md`
**Contexte :** `teams` — lecture depuis l'agrégat

## Objectif

Afficher la section staff (relances, apothicaire, assistants, cheerleaders) dans la fiche d'équipe
à partir des champs typés de l'agrégat `Team`, sans aucune dépendance vers un autre BC.

---

## Conception

### `StaffLineVm` et `StaffVm`

View models uniquement — les primitifs y sont autorisés (couche présentation).

```rust
pub struct StaffLineVm {
    pub label:    String,
    pub quantity: u8,   // vue model : primitif autorisé ici
}

pub struct StaffVm {
    pub lines: Vec<StaffLineVm>,
}
```

Construit depuis l'agrégat, en lisant les value objects :

```rust
impl StaffVm {
    pub fn from(team: &Team) -> Self {
        let mut lines = Vec::new();
        if team.rerolls.0      > 0 { lines.push(StaffLineVm { label: "Relances".into(),    quantity: team.rerolls.0 }); }
        if team.apothecaries.0 > 0 { lines.push(StaffLineVm { label: "Apothicaire".into(), quantity: team.apothecaries.0 }); }
        if team.assistants.0   > 0 { lines.push(StaffLineVm { label: "Assistants".into(),  quantity: team.assistants.0 }); }
        if team.cheerleaders.0 > 0 { lines.push(StaffLineVm { label: "Cheerleaders".into(), quantity: team.cheerleaders.0 }); }
        Self { lines }
    }
}
```

### Intégration dans `TeamDetailVm`

```rust
pub struct TeamDetailVm {
    // … champs existants …
    pub staff: StaffVm,
}
```

### Template `teams-team-detail.html`

Remplacer le slot de chargement des joueurs par la table staff réelle :

```html
{% if !vm.staff.lines.is_empty() %}
<div class="staff-panel">
  <div class="staff-panel-title">Staff</div>
  <table class="player-table staff-table">
    <thead>
      <tr>
        <th style="text-align: left;">Poste</th>
        <th style="text-align: center;">Quantité</th>
      </tr>
    </thead>
    <tbody>
      {% for line in vm.staff.lines %}
      <tr>
        <td><strong>{{ line.label }}</strong></td>
        <td style="text-align: center;">{{ line.quantity }}</td>
      </tr>
      {% endfor %}
    </tbody>
  </table>
</div>
{% endif %}
```

---

## Checklist

- [ ] `StaffLineVm` + `StaffVm` dans `teams/io/web/team_detail.rs`
- [ ] `StaffVm::from(&team)` : lit les `.0` des value objects de l'agrégat
- [ ] `TeamDetailVm` enrichi du champ `staff: StaffVm`
- [ ] `TeamDetailVm::from()` : appel à `StaffVm::from(&team)`
- [ ] Template `teams-team-detail.html` : section staff câblée, masquée si vide
