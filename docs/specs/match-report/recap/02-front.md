# Récap — Phase 2 : Architecture front

## Vue d'ensemble

Page de consultation pure. Pas de composition multi-widgets : un seul handler assemble toutes les données côté serveur et produit le template complet.

La page a deux états selon le variant d'état du match report :
- `ReadyToPublish` → CTA "Publier" + "← Modifier étape 5"
- `Published` → CTA "Retour compétition" + "Voir fiche [équipe home]"

## Endpoints

| Handler | Méthode | URL | Rôle |
|---|---|---|---|
| `get_recap` | GET | `/app/{space_id}/match-report/{mr_id}/recap` | Affiche le récap complet |
| `post_publish` | POST | `/app/{space_id}/match-report/{mr_id}/recap/publish` | Publie définitivement le rapport |

Accès : utilisateur connecté obligatoire (middleware auth existant). Accès public en temps 2.

## Sources de données (assemblées dans le handler GET)

| Donnée | Source |
|---|---|
| État du match report (actions, gains, fan_mod, résumé…) | Repo local — `MatchReportReadyToPublish` ou `MatchReportPublished` |
| Noms d'équipes, coach, logo, roster_id | `ITeamDataPort::find_team_info` (existant) |
| Contexte compétition (noms compét./saison/journée) | `ICompetitionDataPort::find_round_context` (nouvelle méthode) — dégradation gracieuse si absent |
| SPP par joueur ce match | `ISppCalculatorPort::calculate_match_spp` (nouveau port, mini BC SppCalculator) |

## Mini BC SppCalculator

BC autonome responsable du calcul des SPP gagnés par joueur sur la base des actions d'un match et des règles SPP du roster (issues de BC References).

Interface exposée au BC Match Report via port :

```
ISppCalculatorPort
  calculate_match_spp(home_actions, away_actions, home_roster_id, away_roster_id)
    → SppMatchResult { home: Vec<(ActionPlayer, u8)>, away: Vec<(ActionPlayer, u8)> }
```

Internalement : fetche les `SppRules` depuis BC References (IRosterSppPort), puis applique une fonction pure de calcul.

## Interactions utilisateur

| Action | Mécanisme | Résultat |
|---|---|---|
| Chargement de la page | GET classique (pas HTMX) | Template complet rendu |
| Clic "Publier" | `<form method="post">` vers `post_publish` | Redirect vers `get_recap` (état Published) |
| Clic "← Modifier étape 5" | `<a href>` vers step5 | Navigation classique |
| Clic "Retour compétition" | `<a href>` vers page compétition | Navigation classique |

Pas de HTMX partiel, pas de communication inter-widgets. Page statique une fois chargée.

## Règles d'accès par état

| État du match report | GET recap | POST publish |
|---|---|---|
| `Draft` | 404 | 404 |
| `PreMatch` | 404 | 404 |
| `ReadyToPublish` | ✅ (avec CTA Publier) | ✅ |
| `Published` | ✅ (avec CTA Retour) | 409 Conflict |
| `Cancelled` | 410 Gone | 410 Gone |

## Règles métier identifiées

- Publication irréversible : une fois `Published`, le rapport ne peut plus être modifié
- La publication émet un AppEvent `MatchReportPublished` (payload complet) consommé par BC Teams et BC Players
- Le calcul SPP est propre à ce match (SPP gagnés dans le match, pas le total du joueur)
- Les `Blesse { injury }` ne génèrent pas de SPP (actions subies) — seuls les `Sortie` (infligés) en génèrent
- Les `SppRules` dépendent du roster (TD SPP et Sortie SPP varient selon le type d'équipe)
