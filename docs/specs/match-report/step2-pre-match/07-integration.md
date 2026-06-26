# Step 2 — Avant-match — Intégration

## Persistance

### Event store

L'événement `FanFactorRecorded` est persisté dans `match_report_event_store` via `repo.append()`. Même mécanique que les événements existants.

### Projection

Pas de mise à jour de `match_report_projection` pour cet événement — le fan factor n'est pas une donnée de projection (pas besoin de le requêter en liste). Il est reconstruit par rehydratation de l'agrégat.

Dans `update_projection_in_tx`, ajouter un bras vide :

```rust
MatchReportDomainEvent::FanFactorRecorded { .. } => {}
```

## Événements

Aucun app event émis. Le fan factor est interne au match report.

## Handlers

### GET `/app/{space_id}/match-report/{match_report_id}/step2`

**Extracteurs** : `AuthSession`, `Path(space_id, match_report_id)`, `State`

**Logique** :
1. Vérifier l'authentification
2. Charger le match report via `repo.find_by_id()`
3. Si état != PreMatch → redirect vers la page appropriée (Draft → step1, Cancelled → 410)
4. Construire les URLs JSON du BC Teams pour les deux équipes
5. Rendre `PreMatchTemplate`

**Retour** : `impl IntoResponse` (HTML)

### POST `/app/{space_id}/match-report/{match_report_id}/step2`

**Extracteurs** : `AuthSession`, `Path(space_id, match_report_id)`, `State`, `Form(RecordFanFactorForm)`

**Logique** :
1. Vérifier l'authentification
2. Valider les D3Roll via smart constructors (400 si invalide)
3. Construire `RecordFanFactorCommand`
4. Appeler `record_fan_factor_use_case::execute()`
5. Redirect vers step 2 inducements (ou step 3 si pas d'inducements — à voir plus tard)

**Retour** : `Redirect`

### GET `/app/{space_id}/team/widgets/match-context/json?team_id=XXX`

**Extracteurs** : `Path(space_id)`, `Query(team_id)`, `State`

**Logique** :
1. Charger les données d'équipe depuis le repository Teams
2. Construire `TeamMatchContextJson`
3. Retourner en JSON

**Retour** : `Json<TeamMatchContextJson>`

## Templates

### `pre-match.html`

Extends `app-layout.html`. Structure :
- Header "Séquence d'avant-match"
- Steps indicator (step 1 done, step 2 active)
- Match banner (noms/coaches/rosters rendus côté serveur)
- Sections fan factor, journaliers, TV, inducements (alimentées côté client via `fetch()`)
- Formulaire POST avec les deux inputs D3
- Boutons retour/suivant

Le template utilise Alpine `x-data` pour :
- Stocker les données d'équipe chargées via `fetch()`
- Calculer les totaux fan factor en temps réel
- Calculer la différence de TV et l'ordre d'inducements

### Chargement des données d'équipe

```javascript
// Au mount Alpine
async init() {
  const [home, away] = await Promise.all([
    fetch(this.homeUrl).then(r => r.json()),
    fetch(this.awayUrl).then(r => r.json())
  ]);
  this.home = home;
  this.away = away;
}
```

## Routes

### BC match_report

| Constante | Path |
|---|---|
| `MATCH_REPORT_STEP2` | `/app/{space_id}/match-report/{match_report_id}/step2` |

Builder : `step2(&self, space_id, match_report_id) -> String`

### BC Teams

| Constante | Path |
|---|---|
| `TEAM_MATCH_CONTEXT_JSON` | `/app/{space_id}/team/widgets/match-context/json` |

Builder : `team_match_context_json(&self, space_id) -> String`

## Tests E2E prévus

1. **Accès step 2** — naviguer vers step 2 d'un match report en PreMatch → page affichée avec les deux équipes
2. **Fan factor** — saisir les D3 rolls, vérifier le calcul des totaux en temps réel
3. **Soumission** — soumettre le formulaire → redirect vers la page suivante
4. **Soumission invalide** — D3 hors {1,2,3} → erreur
5. **Accès en Draft** — tenter d'accéder à step 2 sur un rapport en Draft → redirect step 1
6. **Données équipe** — vérifier que les sections journaliers/TV/inducements affichent les bonnes données
