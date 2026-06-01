# BC `match_report` — Widget V/N/D d'une équipe

**Priorité : moyenne**
**Dépend de :** BC `match_report` (non encore créé)
**Contexte :** `match_report` (émetteur du widget) → `teams` + `team_creation` (consommateurs)

## Objectif

Exposer un fragment HTML affichant le bilan victoires / nuls / défaites d'une équipe, utilisable comme widget HTMX dans la fiche d'équipe (BC `teams`) et les cartes de la page "Mes équipes" (BC `team_creation`).

---

## Utilisation prévue

```html
<!-- Dans team-detail.html (BC teams) -->
<div hx-get="{{ match_report_routes.vnd_widget(space_id, team_id) }}"
     hx-trigger="load"
     hx-target="this"
     hx-swap="innerHTML">
</div>

<!-- Dans my-teams-widget.html (BC teams), pour chaque carte active -->
<div hx-get="{{ match_report_routes.vnd_widget(space_id, team_id) }}"
     hx-trigger="load"
     hx-target="this"
     hx-swap="innerHTML">
</div>
```

---

## Ce qui reste à définir

- Comment BC `match_report` stocke-t-il les résultats ? Agrégat event sourcé ? Table de résultats ?
- Le bilan est-il global (toutes saisons) ou limité à la saison courante ? La fiche d'équipe affiche visiblement le bilan de la saison en cours.
- Le widget affiche-t-il uniquement V/N/D, ou aussi des statistiques complémentaires (TD marqués/encaissés, cas occasionnés) ?
- Endpoint attendu :
  ```
  GET /app/{space_id}/match-report/teams/{team_id}/vnd-widget[?season_id=...]
  ```

## Ébauche du fragment rendu

```html
<div class="team-record">
  <div class="record-item">
    <div class="record-val win">8</div>
    <div class="record-label">V</div>
  </div>
  <div class="record-item">
    <div class="record-val draw">2</div>
    <div class="record-label">N</div>
  </div>
  <div class="record-item">
    <div class="record-val loss">1</div>
    <div class="record-label">D</div>
  </div>
</div>
```

---

## Checklist (à compléter après création du BC `match_report`)

- [ ] Définir le modèle de persistance des résultats dans BC `match_report`
- [ ] Décider bilan global vs bilan par saison + paramètre `season_id`
- [ ] Endpoint `GET /app/{space_id}/match-report/teams/{team_id}/vnd-widget`
- [ ] Fragment template minimal (V/N/D + styles CSS existants)
- [ ] Intégrer le slot `hx-get` dans `team-detail.html` (carte 34)
- [ ] Intégrer le slot `hx-get` dans `my-teams-widget.html` (carte 44)
