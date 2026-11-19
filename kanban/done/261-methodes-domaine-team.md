# Méthodes domaine de `Team` — achat et renvoi

**Priorité : haute**
**Dépend de :** 255
**Bloque :** 263, 265, 268, 270
**Spec :** `recrutement/06-domaine.md` §4, `renvois/06-domaine.md` §2
**Fichiers :** `src/app/teams/domain/team.rs`, `src/app/teams/domain/error.rs`

## Problème

`Team` ne sait ni recruter ni licencier. `PlayerRecruited`, `PlayerFired` et
`PlayerNotReEngaged` sont définis, appliqués dans `apply()`, et **construits nulle
part** — aucune méthode ne les produit.

Et deux méthodes existantes sont fausses au regard des règles validées :

- **`buy_staff` refuse l'apothicaire** (`StaffTypeNotBuyable`) alors qu'il est
  achetable pour les rosters qui y ont droit
- **`dismiss_staff` refuse la relance** (`StaffTypeNotDismissable`) alors qu'elle est
  renvoyable

L'asymétrie saute aux yeux : l'apothicaire est aujourd'hui renvoyable mais pas
achetable, la relance achetable mais pas renvoyable.

## Action

### 1. Recruter

```rust
pub fn recruit_player(&self, position: PositionId, base_value: Kpo, cost: Kpo)
    -> Result<TeamDomainEvent, DomainError>
```

Gardes : phase `Recruitment`, trésorerie suffisante.

La vérification de trésorerie est **redondante** avec le contrôle en total du brouillon
— le total garantit que les débits successifs ne passent jamais sous zéro — mais elle
protège l'invariant propre à `Team` : sa trésorerie n'est jamais négative. On la garde
comme filet.

`Team` ne vérifie **ni** le plafond de 16, **ni** les quotas, **ni** les limites
croisées : il ne connaît pas la composition de l'effectif. Ces gardes vivent dans le
brouillon (cartes 262, 267), qui porte l'effectif hydraté.

### 2. Licencier

```rust
pub fn dismiss_player(&self, player: PlayerId, value_at_dismissal: Kpo)
    -> Result<TeamDomainEvent, DomainError>
```

Garde : phase `Dismissals` uniquement. Le plancher des 11 éligibles est vérifié par le
brouillon, qui connaît les disponibilités.

`value_at_dismissal` est conservée bien qu'elle ne serve à aucun calcul : elle
documente ce que valait le joueur au moment du renvoi, information non reconstructible
une fois qu'il a quitté l'effectif.

**Renommer `PlayerFired` en `PlayerDismissed`** — jamais émis, renommage gratuit, et
il aligne le vocabulaire sur la phase `Dismissals` et sur `players` (carte 260).

### 3. Corriger `buy_staff`

Autoriser l'apothicaire. Le facteur fans reste refusé. La condition `allowed_staff` du
roster **reste dans le brouillon** : `Team` ne connaît pas son catalogue.

### 4. Corriger `dismiss_staff`

Autoriser la relance. Le facteur fans reste refusé. `refund_kpo` a déjà disparu avec la
carte 255.

### 5. Nouvelles variantes de `DomainError`

`MaxPlayersReached`, `PositionQuotaReached`, `CrossLimitExceeded`,
`PositionNotInRoster`, `StaffNotAllowedForRoster`, `StaffQuotaReached`,
`EligibleFloorReached`, `DraftLineNotFound`, `PlayerNotInSquad`.

Elles sont levées par les brouillons (cartes 262 et 267) mais déclarées ici, avec leur
`Display`.

### 6. Bras `apply()` devenus vides

Une fois la carte 251 passée, `PlayerFired` et `PlayerNotReEngaged` n'ont plus de bras
— ils ne faisaient que muter `team_value`. Trancher : les supprimer par cohérence avec
« on supprime le code mort », ou les garder comme contrat. `PlayerDismissed` sera émis,
donc il reste ; `PlayerNotReEngaged` ne l'est toujours pas.

## Checklist

- [ ] `recruit_player` : phase + trésorerie, rien d'autre
- [ ] `dismiss_player` : phase uniquement
- [ ] `PlayerFired` renommé `PlayerDismissed`
- [ ] `buy_staff` accepte l'apothicaire, refuse le facteur fans
- [ ] `dismiss_staff` accepte la relance, refuse le facteur fans
- [ ] Les 9 variantes de `DomainError` déclarées avec leur `Display`
- [ ] Sort de `PlayerNotReEngaged` tranché explicitement
- [ ] Tests 18 à 23 de `recrutement/06-domaine.md` et 12 à 16 de `renvois/06-domaine.md`
- [ ] `make check-arch` au vert, `make test` au vert
