# BC `competitions` — Tests E2E dashboard admin

**Priorité : haute**
**Dépend de :** carte 84 (fragment dashboard)
**Contexte :** BC `competitions` — administration de compétition
**Spec :** `docs/specs/competition-admin/dashboard/07-integration.md`

## Objectif

Couvrir le dashboard admin par des tests E2E Playwright : accès autorisé/refusé, présence des éléments, navigation entre onglets.

---

## Fichier à créer

| Fichier | Rôle |
|---|---|
| `tests/e2e/test_competition_admin_dashboard.py` | Tests E2E Playwright |

## Scénarios

### 1. Accès admin autorisé

- Se connecter en tant qu'admin de la compétition
- Naviguer vers `/app/{space_id}/competitions/{competition_id}/{season_id}/admin`
- Vérifier que la page se charge : banner visible (`.admin-banner`), tabs visibles (`.admin-tabs`), contenu dashboard visible

### 2. Accès refusé (non-admin)

- Se connecter en tant que coach non-admin
- Naviguer vers la même URL
- Vérifier une réponse 403

### 3. Stats présentes

- Accéder au dashboard
- Vérifier que les chips de stats sont visibles (`.stat-chip` — au moins 3 éléments)

### 4. Navigation entre onglets

- Accéder au dashboard
- Cliquer sur l'onglet "Inscriptions"
- Vérifier que le contenu de `#admin-content` change
- Vérifier que l'URL est mise à jour

---

## Checklist

- [ ] Créer `test_competition_admin_dashboard.py`
- [ ] Scénario 1 : accès admin autorisé → page chargée
- [ ] Scénario 2 : accès refusé → 403
- [ ] Scénario 3 : stats chips visibles
- [ ] Scénario 4 : navigation onglets fonctionne
- [ ] Tous les tests passent (`make e2e`)
