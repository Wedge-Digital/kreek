# Step 1 — Sélection du match : Architecture front

## Type de page

Formulaire de sélection en cascade (selects dépendants). Pas de pattern "page à widgets" — un seul handler rend la page, avec des endpoints HTMX pour les selects dépendants.

## Deux modes d'accès

### Mode 1 — Formulaire vierge

URL : `GET /app/{space_id}/match-report/new`

Le coach ou admin arrive sur un formulaire vide et sélectionne manuellement compétition → saison → journée → équipes.

### Mode 2 — Rapport pré-créé (depuis pairing)

URL : `GET /app/{space_id}/match-report/{match_report_id}`

Le match report a été créé automatiquement par un app event émis par le BC competitions lors de la programmation du pairing. Le formulaire est pré-rempli. L'utilisateur vérifie et continue.

Les deux modes convergent vers le même template avec les mêmes composants.

## Composants de la page

### Selects en cascade

| Select | Source données | Déclenche | Pré-rempli (mode 2) |
|--------|---------------|-----------|---------------------|
| Compétition | Compétitions de l'espace avec saison active | Rechargement du select Saison | Oui |
| Saison | Saisons de la compétition sélectionnée (ordre anti-chronologique, dernière saison pré-sélectionnée) | Rechargement du select Journée | Oui |
| Journée | Journées de la saison sélectionnée | Rechargement des selects Équipes | Oui |
| Équipe domicile | Équipes enrolled dans la saison | Mise à jour de la carte preview | Oui |
| Équipe visiteur | Équipes enrolled dans la saison | Mise à jour de la carte preview | Oui |

Tous les selects utilisent TomSelect searchable (convention projet). Pour les selects d'équipe, la recherche porte sur le nom de l'équipe et le nom du coach.

### Cartes de preview équipe

Sous chaque select d'équipe, une carte affiche les infos de l'équipe sélectionnée :
- Logo/initiales
- Nom de l'équipe
- Coach, roster, TV

Mise à jour via un fragment HTMX déclenché par le `change` du select équipe correspondant, ou via JS local (les données sont dans les options TomSelect).

### Bannière pré-remplissage

Affichée uniquement en mode 2. Message : "Match pré-sélectionné depuis le calendrier — vérifiez et continuez."

### Message d'erreur

Affiché en réponse au POST si la validation échoue (ex. : même équipe sélectionnée deux fois, équipe non enrolled).

## Endpoints HTMX

| Endpoint | Méthode | Trigger | Réponse |
|----------|---------|---------|---------|
| `.../match-report/new` | GET | navigation | Page complète step1 |
| `.../match-report/{id}` | GET | navigation | Page complète step1 pré-remplie |
| `.../match-report/new/seasons` | GET | `change` sur select compétition | Fragment : options `<option>` pour select saison |
| `.../match-report/new/rounds` | GET | `change` sur select saison | Fragment : options `<option>` pour select journée |
| `.../match-report/new/teams` | GET | `change` sur select journée | Fragment : options pour selects home/away |
| `.../match-report/new` | POST | clic "Commencer" | Crée le match report, redirect vers step2 |
| `.../match-report/{id}` | POST | clic "Commencer" (mode 2) | Valide/met à jour, redirect vers step2 |

### Params des endpoints de cascade

- `GET .../seasons?competition_id=X`
- `GET .../rounds?season_id=X`
- `GET .../teams?season_id=X` (toutes les enrolled de la saison, sans filtre poule)

## Comportement selon le rôle

### Coach lambda

- Le select compétition ne liste que les compétitions où il a au moins une équipe enrolled
- Le select saison ne liste que les saisons où il a au moins une équipe enrolled
- Le select "mon équipe" est restreint à ses propres équipes enrolled dans la saison
- Le select "adversaire" propose toutes les autres équipes enrolled (sans filtre par poule)
- Si le coach n'a qu'une seule équipe enrolled dans la saison, elle est auto-sélectionnée

### Admin compétition / Admin espace

- Toutes les compétitions de l'espace sont listées
- Les deux selects d'équipe proposent librement toutes les équipes enrolled dans la saison
- Pas de distinction home/away imposée

## Interactions front

### Cascade de selects

Tout est géré par HTMX, pas de JS d'orchestration :

1. `change` sur compétition → `hx-get` recharge les options saison (et vide journée + équipes)
2. `change` sur saison → `hx-get` recharge les options journée (et vide équipes)
3. `change` sur journée → `hx-get` recharge les options équipes

### Cartes de preview

Les cartes de preview équipe sont mises à jour côté client par le `onChange` de TomSelect (les données coach/roster/TV sont dans les options du select, pas besoin d'un aller-retour serveur).

### Validation au POST

Le bouton "Commencer" fait un POST classique (pas HTMX). En cas d'erreur, la page est re-rendue avec le message d'erreur visible et les selects dans leur état précédent. En cas de succès, redirect HTTP vers step2.

## Pas de widgets, pas d'événements DOM

Cette page est un formulaire classique en cascade. Pas de communication inter-widgets, pas d'événements DOM sur `body`. Le pattern "page d'assemblage à widgets" ne s'applique pas.

## BC propriétaire

**BC `match_report`** (nouveau BC). Les données de compétitions/saisons/journées et d'équipes sont obtenues via des **ports** définis dans le BC match_report :

- Port compétitions : lister compétitions, saisons, journées
- Port équipes : lister les équipes enrolled avec leurs infos (nom, coach, roster, TV)

## Règles métier identifiées (step1)

1. Seules les compétitions avec au moins une saison active et un calendrier sont listées
2. Les saisons sont affichées en ordre anti-chronologique — la dernière saison est pré-sélectionnée par défaut
3. Toutes les journées de la saison sont proposées (pas de filtre "déjà jouée")
4. Toutes les équipes enrolled **et en phase `ReadyToPlay`** dans la saison sont proposées, **sans filtre par poule** — les équipes en `MatchReporting` ou autre phase ne sont pas sélectionnables
5. Les deux équipes sélectionnées doivent être différentes
6. Les deux équipes doivent être enrolled et en `ReadyToPlay` dans la saison sélectionnée
7. Coach lambda : ne peut sélectionner que ses propres équipes en tant que "son" équipe
8. Admin : choix libre des deux équipes
9. En mode pré-rempli (pairing), le match report existe déjà — le POST met à jour, pas de création
