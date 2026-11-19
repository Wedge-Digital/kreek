# Composant partagé `team-card.html` — initiales + nouveaux statuts

**Priorité : haute**
**Dépend de :** rien de nouveau
**Contexte :** `src/web/templates/components/` (composant partagé) — impacte `teams` et `team_creation`

## Objectif

Le composant partagé `team-card.html` n'affiche rien quand une équipe n'a
pas de logo (cercle vide). La maquette `app-my-teams.html` affiche des
initiales en repli. Ajouter ce comportement, et étendre le jeu de badges de
statut du composant pour couvrir les valeurs utilisées par le futur widget
"Mes équipes" (carte 287) : aujourd'hui seuls `draft`/`active` existent.

Askama ne supporte pas les paramètres optionnels sur les macros : le
changement de signature impacte **tous** les appelants existants.

**Spec de référence :** `docs/specs/my-teams/mes-equipes/04-dtos.md` et `07-integration.md`.

---

## Conception

### Signature

```
{% macro card(name, logo, roster, coach, tv, initials, status, status_label, link) %}
```

Rendu du fallback :
```html
<div class="team-card-logo">
    {% if let Some(url) = logo %}
    <img src="{{ url }}" alt="{{ name }}">
    {% else %}
    <span class="team-card-initials">{{ initials }}</span>
    {% endif %}
</div>
```

### Appelants à mettre à jour

- `src/app/teams/io/web/templates/widgets/competition-teams.html` — calcule
  et passe ses propres initiales (nouvel usage, pas demandé par cette
  feature mais requis par le changement de contrat).
- `src/app/team_creation/io/web/templates/my-teams.html` — sera de toute
  façon réécrit par la carte 288 (n'utilisera plus ce macro pour les
  brouillons) ; pas de mise à jour ad hoc nécessaire ici si les deux cartes
  sont livrées dans l'ordre.

### CSS — nouvelles variantes de statut

```css
/* assets/static/css/components/team-card.css — ajouts */
.team-card-status--pending   { background: rgba(255,107,53,0.12);  color: var(--orange); }
.team-card-status--ready     { background: rgba(98,149,132,0.15); color: var(--green); }
.team-card-status--phase     { background: rgba(0,48,73,0.10);    color: var(--main-blue); }
.team-card-status--offseason { background: var(--dark-6);         color: var(--dark-3); }
.team-card-initials {
  font-family: var(--font-title); font-weight: 800; font-size: 14px; color: var(--dark-2);
}
```
Couleurs identiques à `.team-status-badge--*` de `app-team-detail.css` — pas de nouvelle palette inventée.

---

## Checklist

- [ ] Paramètre `initials` ajouté au macro `card(...)`
- [ ] Fallback initiales rendu quand `logo` est `None`
- [ ] `competition-teams.html` mis à jour (calcul + passage des initiales)
- [ ] CSS : 4 nouvelles variantes de statut + style `.team-card-initials`
