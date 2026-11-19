> **Carte annulée le 2026-08-29 — remplacée par les cartes 476 à 478.**
>
> Ses deux prémisses étaient périmées. « **Dépend de :** BC `match_report` (non
> encore créé) » — il existe. Et sa checklist demandait un « modèle de
> persistance des matchs dans BC `match_report` » : **il n'y a rien à
> modéliser.** `competition_match_display_proj` porte déjà, par match, les
> identifiants et les noms dénormalisés des deux équipes, leurs rosters, leurs
> coachs, leurs logos, le score, les blessures et l'URL du rapport.
>
> Elle se trompait donc aussi de BC : cette table appartient à `competitions`,
> qui sert le fragment ; la fiche d'équipe le compose. C'est la même erreur que
> la carte 13 sur les statistiques — demander un ticket de modélisation pour
> une donnée présente depuis l'origine.
>
> Ses quatre questions ouvertes sont tranchées : saison courante seulement,
> matchs à venir compris, une pastille V/N/D du point de vue de l'équipe, et le
> lien de rapport pour le coach de l'équipe et les admins. La conception vit
> dans `docs/specs/matchs-d-une-equipe/`.

# BC `match_report` — Onglet Matchs de la fiche d'équipe

**Priorité : moyenne**
**Dépend de :** BC `match_report` (non encore créé)
**Contexte :** `match_report` (émetteur du widget) → `teams` (consommateur)

## Objectif

Exposer un fragment HTML listant l'historique des matchs d'une équipe, chargé via HTMX dans l'onglet "Matchs" de la fiche d'équipe (BC `teams`).

---

## Utilisation prévue

```html
<!-- Dans team-detail.html (BC teams), onglet Matchs -->
<div id="matches-tab"
     hx-get="{{ match_report_routes.team_matches_widget(space_id, team_id) }}"
     hx-trigger="click from:#tab-matches"
     hx-target="#tab-content"
     hx-swap="innerHTML">
</div>
```

---

## Ce qui reste à définir

- Quelles informations par match : date, adversaire, résultat (V/N/D), score (TD), cas occasionnés ?
- Les matchs sont-ils filtrables par saison ?
- Ordre d'affichage : chronologique inversé (plus récent en haut) ?
- Endpoint attendu :
  ```
  GET /app/{space_id}/match-report/teams/{team_id}/matches-widget[?season_id=...]
  ```

---

## Checklist (à compléter après création de BC `match_report`)

- [ ] Modèle de persistance des matchs dans BC `match_report`
- [ ] Endpoint `GET …/matches-widget`
- [ ] Fragment template : tableau de matchs (date, adversaire, score, résultat)
- [ ] Filtre optionnel par saison
- [ ] Intégrer le slot `hx-get` dans `team-detail.html` (carte 34)
