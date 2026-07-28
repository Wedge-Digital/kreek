# Use cases de renvois

**Priorité : haute**
**Dépend de :** 256, 261, 267
**Bloque :** 269
**Spec :** `docs/specs/phases-recrutement-renvois/renvois/05-use-cases.md`
**Fichiers :** `src/app/teams/use_cases/mark_player_for_dismissal_use_case.rs`,
`mark_staff_for_dismissal_use_case.rs`,
`validate_dismissals_phase_use_case.rs`, `src/app/teams/use_cases/commands.rs`

## Problème

Marquer un joueur, marquer du staff, appliquer le lot : aucune de ces orchestrations
n'existe. `validate_dismissals_phase_use_case` existe mais ne fait que valider la
transition de phase.

## Action

### 1. Les use cases de marquage

Même forme qu'au recrutement : charger `Team` → vérifier la phase `Dismissals` →
hydrater → `draft.mark_player(id)?` → persister avec la version.

**Le démarquage réutilise `remove_draft_line_use_case`** (carte 263) : retirer une
ligne d'un brouillon par son identifiant est la même opération quelle que soit la
phase.

L'hydratation réutilise le domain service de la carte 263, avec un brouillon de phase
`Dismissals`.

### 2. La validation — trois différences avec le recrutement

**Aucune vérification de trésorerie** : rien n'entre, rien ne sort. `validate_all()` ne
contrôle que le plancher des 11 éligibles et la possession du staff marqué.

**Le lot** : un `PlayerDismissed` par joueur, un `StaffDismissed` par ligne de staff
(sans `refund_kpo`, supprimé par la carte 255), `DismissalsPhaseValidated` en dernier.

**Aucun mouvement de trésorerie n'est produit.** `treasury_movement()` retourne `None`
pour les deux événements de renvoi — c'est la traduction en code, **vérifiée par le
compilateur**, de « un renvoi ne rembourse rien ».

### 3. La garde anti-double-application fonctionne à l'identique

`DismissalsPhaseValidated` fait passer l'équipe en `ReadyToPlay`, et
`validate_dismissals_phase()` exige la phase `Dismissals`. Une revalidation échoue,
donc la suppression du brouillon peut rester hors transaction.

### 4. Une conséquence heureuse

Ce dernier événement déclenche **aussi** la purge des brouillons (carte 257) et le
recalcul de valeur d'équipe (carte 251), tous deux abonnés aux entrées en
`ReadyToPlay`.

La valeur d'équipe est donc recalculée après le renvoi **sans aucun ordonnancement
explicite** — sous réserve de la course traitée par la carte 270.

### 5. Erreur supplémentaire

`EligibleFloorReached` → 422. En pratique elle ne devrait pas survenir, le bouton étant
déjà désactivé — sauf en cas de version périmée, où elle fait office de **deuxième
barrière**.

## Checklist

- [ ] Les 2 use cases de marquage sans logique métier
- [ ] Démarquage par `remove_draft_line_use_case`, non dupliqué
- [ ] Validation : aucune vérification de trésorerie
- [ ] Lot : un événement par ligne + transition en dernier
- [ ] Test : `treasury_movement` retourne `None` pour les deux événements
- [ ] Test : revalider après succès → `WrongPhase`
- [ ] Test : brouillon vide → seul `DismissalsPhaseValidated` est appendu
- [ ] `make check-arch` au vert, `make test` au vert
