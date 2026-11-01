# Le journal des deux camps

**Épic :** E21 — Le mode de saisie et l'après-match en séquence · **Ordre :** 5 · **Dépend de :** 634
**Conception :** `docs/specs/saisie-mobile/E21-cadrage.md` (R6, § 5)
**Maquette :** `assets/rawpages/html/app-mobile-solo-live.html`, `app-mobile-solo-follow.html`

## Objectif

Le journal montre **les deux camps** dans un seul fil, du plus récent au plus
ancien ; chaque action dit qui l'a saisie (« saisi par toi », « saisi par
Yann ») ; ✕ n'apparaît que pour qui peut écrire. Aujourd'hui il lit un camp
(`find_actions_by_match_and_side`, `io/repository/match_report_repository.rs:537-575`)
et porte ✕ sur chaque ligne sans condition (`templates/action-log-widget.html`).

## Conception

- **Projection** : colonne `match_report_actions.recorded_by TEXT`, écrite à
  l'insertion depuis `ActionRecorded.recorded_by`
  (`match_report_repository.rs:181-224`), et remplie pour l'existant par une
  migration SQL (ci-dessous). Nullable : une action sans événement resterait
  sans auteur plutôt que d'en recevoir un inventé.
- **Dépôt** : `find_actions_by_match` rend les deux camps avec leur camp et leur
  auteur ; il remplace `find_actions_by_match_and_side` (consommateurs listés
  avant suppression, règle 4).
- **Widget** : une route `GET /{id}/action-log`, dans `routes_de_suivi()` (634).
  Elle remplace `step3/action-log` et `step4/action-log`. Chaque ligne : tour,
  icône, joueur, action, équipe, « saisi par » — `ICoachDataPort::find_coach_name`,
  « toi » pour l'utilisateur courant. ✕ si `can_write_report` (634).
- `actions-step.html` charge le nouveau widget.

## Données de production

**Une colonne et son remplissage** ; l'event store n'est pas touché.

```sql
ALTER TABLE match_report_actions ADD COLUMN recorded_by TEXT;
UPDATE match_report_actions a
   SET recorded_by = e.payload->>'recorded_by'
  FROM match_report_event_store e
 WHERE e.event_type = 'ActionRecorded'
   AND e.payload->>'action_id' = a.action_id;
```

`recorded_by` est obligatoire dans `ActionRecorded` depuis sa création
(`d1b0d820`), sans `serde(default)` ; aucune migration de données n'a écrit
d'action. **La carte le vérifie sur un dump**, deux comptes à zéro :

```sql
SELECT count(*) FROM match_report_event_store
 WHERE event_type = 'ActionRecorded' AND payload->>'recorded_by' IS NULL;
SELECT count(*) FROM match_report_actions WHERE recorded_by IS NULL;  -- après remplissage
```

## Checklist

- [ ] Migration : colonne et remplissage ; écriture de `recorded_by` à l'insertion
- [ ] `find_actions_by_match` ; l'ancienne lecture retirée après inventaire
- [ ] La route `/action-log`, ses deux anciennes retirées après inventaire ; le widget, « saisi par », ✕ selon le droit
- [ ] Tests unitaires : le VM nomme « toi » et l'autre coach ; ✕ absent pour qui ne peut pas écrire
- [ ] Test d'intégration dépôt : une action insérée porte son auteur
- [ ] E2E : `test_match_report_step3_4_actions.py` suit le journal à deux camps ; l'autre coach voit le journal sans ✕
- [ ] Mesures sur dump consignées
- [ ] `tests/impact-map.toml` mis à jour
- [ ] `make lint`, `make check-arch`, `make test`, e2e impactés
