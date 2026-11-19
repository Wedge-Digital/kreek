# Panneau « Visibilité »

**Épic :** E14 · **Ordre :** 3 · **Dépend de :** 420
**Conception :** `docs/specs/modifier-une-competition/onglet-parametres/`
(`04-dtos.md`, `05-use-cases.md`)

## Objectif

Changer le mode d'accès d'une compétition et son mode de validation des
inscriptions. Le panneau le plus simple des cinq.

## Conception

### Le use case

```rust
pub struct UpdateVisibilitySettingsCommand {
    pub season_id: SeasonId,
    pub access_mode: AccessMode,
    pub requires_validation: RequiresValidation,
}
```

1. `find_invitations(&season_id)` → `SeasonNotFound`
2. remplacer les deux champs, **conserver `invited_coaches`,
   `registration_deadline` et `max_participants`**
3. `find_notifications(&season_id)`
4. `save_invitations(&season_id, &invitations, &notifications)`

**Le seul use case qui relit deux documents.** La signature de
`save_invitations` porte aussi les notifications — héritage de l'étape 4 du
magicien, où les deux se règlent ensemble. Ne pas les relire les remettrait à
leur valeur par défaut, et **les rappels d'échéance s'éteindraient sans que rien
ne le dise**.

`max_participants` traverse sans être touché : le panneau ne l'édite pas (il ne
règle rien, cf. carte 415), mais le réécrire à zéro éteindrait la ligne « il
reste N places » des relances.

### Le handler

```rust
GET  …/settings/visibility  → get_settings_visibility
POST …/settings/visibility  → post_settings_visibility   (axum::Form)
```

```rust
#[derive(Deserialize)]
pub struct VisibilitySettingsForm {
    pub access_mode: String,             // "invitation" | "open"
    pub requires_validation: String,     // "manual" | "automatic"
}
```

Deux chaînes et non deux booléens : ce sont des `<select>` à deux options, et un
libellé futur — « sur candidature » — ne doit pas exiger de changer le type.

**Une valeur inconnue est un `400`**, jamais un repli silencieux sur le défaut,
qui ouvrirait une compétition fermée.

### Le VM

```rust
pub struct VisibilityVm {
    pub access_mode: String,
    pub requires_validation: bool,
    pub invited_count: u32,      // affichage seul
}
```

`invited_count` existe pour une raison : le panneau réécrit `invitations`, dont
`invited_coaches` fait partie. Afficher « 12 coachs invités » rend visible ce que
le POST doit préserver — la relecture cesse d'être une précaution invisible.

## Tests

- Unitaires : les trois champs préservés, les notifications préservées, le refus
  d'une valeur d'énumération inconnue.
- E2E : basculer de « sur invitation » à « ouvert » et vérifier que les coachs
  invités sont toujours là.

## Checklist

- [x] Le use case et ses tests
- [x] Les deux handlers, `require_admin_access`
- [x] Le VM, le template
- [x] `make lint && make test && make check-arch`

## Deux écarts à la conception, validés avant écriture

### `save_visibility` remplace `save_invitations`

**La conception de cette carte portait le défaut de la carte 423.**
`save_invitations` n'écrit pas que les invitations :

```sql
SET invitations = $1, notifications = $2, status = 'invitations_configured'
```

Sur une saison en cours, ce statut la fait régresser sous `ready` ;
`competition_rules_adapter.rs:34` ne la dit plus prête, et la carte 407 interdit
la création d'équipe sur une saison qui ne l'est pas. **Changer un mode d'accès
aurait cassé l'inscription de la compétition entière** — alors que
l'enregistrement, lui, réussit.

Un `update_visibility.sql` n'écrit donc que la colonne `invitations`, sur le
modèle de `update_notifications.sql` et de `save_structure_and_prune_groups`.

### La relecture des notifications disparaît

La carte en faisait son point saillant — « le seul use case qui relit deux
documents ». Cette relecture n'était une précaution que parce que la signature
écrivait les deux colonnes. **Ne pas écrire une colonne ne peut pas la remettre
à son défaut** : le risque disparaît par construction au lieu d'être contourné,
et le use case ne relit plus qu'un document.

### Le template n'a pas une ligne de JS

Deux groupes de `<input type="radio">` habillés en `<label>`, dans un `<form
hx-post>`. Le magicien pilote le même choix en JS ; ici l'état vit dans le
formulaire. Conséquence mesurée : l'e2e tourne en 7 secondes sans navigateur,
contre 15 pour celui de la carte 424.

## Falsification

| Mutation | Constaté |
|---|---|
| **La conception prescrite par la carte** (`save_invitations` + relecture) | `assert 'invitations_configured' == 'ready'` |
| Notifications non relues (`default()`) | « les notifications ont été réécrites » |
| Valeur inconnue repliée sur le défaut | `assert 200 == 400` |
| Invités / échéance / plafond non préservés | 2 tests unitaires rouges |
| Use case empruntant `save_invitations` | test de garde rouge, journal `["save_invitations"]` |

## Ce que la falsification a corrigé dans les tests eux-mêmes

**Le test des notifications ne prouvait rien.** Le défaut du domaine allume les
quatre rappels ; la fixture partait de là. Réécrire la colonne avec `default()`
produisait donc **exactement le même JSON**, et la mutation passait sans que le
test bronche. Il faut que la donnée observée s'écarte du défaut pour qu'une
remise au défaut se voie.

**Et cet écart ne peut pas vivre dans la fixture** : les POST des autres tests
du module remettent la colonne au défaut, donc il n'y survit pas. Le test se le
donne lui-même, juste avant d'observer — sinon il dépend de l'ordre d'exécution,
et échoue sur sa prémisse au lieu de son assertion.
