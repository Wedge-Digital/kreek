# Réglages de notification — domaine et persistance

**Spec :** `docs/specs/notifications/configuration/` (phases 3, 4, 6)
**Dépend de :** rien
**Ouvre :** 332, 333, 336

> **Ne pas livrer seule.** Cette carte fait partie d'une chaîne de dix. Aucune
> carte avant la 340 ne fait partir un email : livrer `configuration/` sans
> `envoi/` produirait un **troisième interrupteur email mort**, mieux dessiné
> que les deux qu'il remplace et tout aussi inerte — le défaut même que cette
> fonctionnalité corrige. Cf. `docs/specs/notifications/README.md`.

## Objectif

Poser le modèle des quatre réglages et leur stockage. Rien de visible pour
l'utilisateur à l'issue de cette carte.

## Conception

### Une quatrième colonne JSONB, pas un champ dans les invitations

`competition_seasons` porte déjà `rules`, `structure`, `invitations`. Une
colonne `notifications` s'ajoute, avec son propre couple select/update.

L'argument décisif est **l'écriture concurrente** : glisser quatre booléens dans
le blob `invitations` obligerait à le lire-modifier-réécrire en entier, donc à
écraser une invitation faite au même moment dans un autre onglet.

L'argument du statut est réel mais plus faible — `update_invitations.sql` écrit
`status = 'invitations_configured'`, ce qui ferait retomber une compétition
vivante dans le magicien — et il n'impose qu'une requête distincte, pas une
colonne.

### La migration remplit les lignes existantes (R8)

```sql
ALTER TABLE competition_seasons ADD COLUMN IF NOT EXISTS notifications JSONB;

UPDATE competition_seasons
SET    notifications = '{"registration_open":false,"round_eve":false,
                         "round_closing":false,"registration_deadline":false}'::jsonb
WHERE  notifications IS NULL;
```

**Ce remplissage n'est pas une commodité.** Sans lui, `NULL` voudrait dire à la
fois « ancienne saison, donc éteint » et « saison neuve, donc allumé », et rien
dans la ligne ne trancherait — `invitations_configured` désigne aussi bien une
saison abandonnée en cours de magicien qu'une saison d'avant la migration.

### `applicability()` ne lit pas les réglages

Ce qui est **coché** et ce qui est **applicable** sont indépendants. Les mêler
rendrait mécaniquement impossible d'afficher une case cochée et grisée — donc de
tenir R6.

## Checklist

- [ ] Migration : colonne `notifications` + remplissage des lignes existantes
- [ ] `domain/competition_notifications.rs` : quatre newtypes booléens
      (`#[serde(transparent)]`, la maison de `competitions`), la struct,
      `#[serde(default)]` valant **allumé**
- [ ] `Inapplicable` et `NotificationApplicability` ; `applicability(structure,
      invitations)` — fonction pure et totale, sans `Result`
- [ ] Cas limites : calendrier activé sans journée = pas de calendrier ; date
      limite `Some("")` = absente
- [ ] `ISeasonRepository` : `find_notifications`, `save_notifications`
- [ ] `select_notifications.sql`, `update_notifications.sql` — **sans `status`**
- [ ] `use_cases/save_competition_notifications.rs`
- [ ] Tests unitaires : les sept cas d'applicabilité de la phase 6, plus le
      round-trip serde (JSON vide → quatre `true`)
- [ ] `save_invitations` **n'est pas touchée** — c'est la carte 333
- [ ] `make check-arch`
