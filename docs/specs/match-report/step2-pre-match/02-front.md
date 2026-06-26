# Step 2 — Avant-match — Architecture front

## Composition de la page

La page est un assemblage de sections dans un conteneur `mr-container`. Pas de widgets HTMX autonomes — c'est une page formulaire unique avec des sections informatives et une saisie (fan factor).

Les données d'équipe (Dedicated Fans, nombre de joueurs, CTV, trésorerie) sont fournies par le BC Teams via un endpoint JSON consommé par la page au chargement.

## Sections

### 1. Match banner (lecture seule)

Affiche les deux équipes du match (nom, coach, roster). Données issues du match report PreMatch (déjà en mémoire côté serveur).

Rendu côté serveur dans le template Askama — pas de widget séparé.

### 2. Fan Factor (saisie)

Pour chaque équipe :
- Dedicated Fans (lecture, donnée BC Teams)
- Jet de D3 (input number, min=1, max=3, saisi par l'utilisateur)
- Total = Dedicated Fans + D3 (calculé en JS côté client, en temps réel)

Le calcul du total est un Alpine `x-data` local. La soumission envoie les deux jets de D3 au backend.

### 3. Journaliers (lecture seule)

Pour chaque équipe, affiche un bandeau :
- Si >= 11 joueurs : "Aucun journalier nécessaire" (bandeau vert)
- Si < 11 joueurs : "X journaliers ajoutés automatiquement (type)" (bandeau bleu)

Données : nombre de joueurs disponibles + type de journeyman du roster. Fournies par le BC Teams.

### 4. Comparaison TV (lecture seule)

Affiche la CTV de chaque équipe et la différence. Bandeau orange si différence > 0, bandeau vert si égalité.

Données : CTV de chaque équipe. Fournies par le BC Teams.

### 5. Inducements order (lecture seule)

Affiche l'ordre d'achat et le budget de chaque équipe :
- Équipe CTV haute : achète 1ère, budget = trésorerie
- Équipe CTV basse : achète 2e, budget = différence CTV + dépenses adverses + trésorerie

Le bouton "Acheter les coups de pouce →" mène à la page step2-inducements.

### 6. Actions (navigation)

Bouton retour (step 1) + bouton suivant (step 2 inducements ou step 3 si pas d'inducements).

## Données nécessaires du BC Teams

Un seul endpoint JSON suffit pour les deux équipes :

`GET /app/{space_id}/team/widgets/match-context/json?team_id=XXX`

Retourne pour une équipe :
```json
{
  "team_id": "...",
  "team_name": "Storm Treemen",
  "coach_name": "Yan",
  "roster_name": "Elfes Sylvains",
  "dedicated_fans": 3,
  "player_count": 12,
  "ctv": 1120,
  "treasury": 150,
  "journeyman_type": "Lineman"
}
```

La page appelle cet endpoint deux fois (home + away) au chargement, via JS `fetch()`, et injecte les données dans les sections.

## Interaction

La page est un formulaire unique. La soumission (`POST`) envoie :
- `home_fan_roll` : jet D3 équipe domicile (1-3)
- `away_fan_roll` : jet D3 équipe visiteur (1-3)

Le backend persiste le fan factor, calcule les journaliers, et redirige vers la page suivante.

## Pas d'événements DOM inter-widgets

C'est une page formulaire simple, pas un assemblage de widgets autonomes. Pas d'événements DOM.
