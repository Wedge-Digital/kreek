# BC `teams` — Affichage du staff dans la fiche d'équipe

**Priorité : moyenne**
**Dépend de :** `51-team-created-staff-transport.md`, `52-team-staff-tracking.md`, `34-team-detail-wired.md`
**Contexte :** `teams` — lecture depuis l'agrégat

## Objectif

Afficher la section staff (relances, apothicaire, assistants, cheerleaders) dans la fiche d'équipe
à partir des données de l'agrégat `Team`, sans aucune dépendance vers un autre BC.

---

## Conception

### `StaffVm`

```rust
pub struct StaffLineVm {
    pub label:    String,
    pub quantity: u8,
}

pub struct StaffVm {
    pub lines: Vec<StaffLineVm>,
}
```

Construit depuis l'agrégat hydraté :

```rust
impl StaffVm {
    fn from(team: &Team) -> Self {
        let mut lines = Vec::new();
        if team.rerolls      > 0 { lines.push(StaffLineVm { label: "Relances".into(),   quantity: team.rerolls }); }
        if team.apothecaries > 0 { lines.push(StaffLineVm { label: "Apothicaire".into(), quantity: team.apothecaries }); }
        if team.assistants   > 0 { lines.push(StaffLineVm { label: "Assistants".into(),  quantity: team.assistants }); }
        if team.cheerleaders > 0 { lines.push(StaffLineVm { label: "Cheerleaders".into(), quantity: team.cheerleaders }); }
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

Remplacer le placeholder par la table staff réelle :

```html
{% if !vm.staff.lines.is_empty() %}
<div class="staff-panel">
  <div class="staff-panel-title">Staff</div>
  <table class="player-table staff-table">
    <thead>
      <tr>
        <th>Poste</th>
        <th>Quantité</th>
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

- [ ] `StaffVm` + `StaffLineVm` dans `teams/io/web/team_detail.rs`
- [ ] `StaffVm::from(&team)` : construit depuis les champs staff de l'agrégat
- [ ] `TeamDetailVm` enrichi du champ `staff: StaffVm`
- [ ] Template `teams-team-detail.html` : section staff câblée (masquée si vide)
- [ ] `TeamDetailVm::from()` : appel à `StaffVm::from(&team)`
