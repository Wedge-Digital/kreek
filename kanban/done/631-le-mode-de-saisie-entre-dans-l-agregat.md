# Le mode de saisie entre dans l'agrégat

**Épic :** E21 — Le mode de saisie et l'après-match en séquence · **Ordre :** 1 · **Dépend de :** —
**Conception :** `docs/specs/saisie-mobile/E21-cadrage.md` (R1, R2, § 3)

## Objectif

Le compte rendu sait dire **dans quel mode il se saisit et qui l'a démarré**.
Aujourd'hui, aucun état ne retient qui a confirmé : le rejeu lit
`SelectionConfirmed` sans son `confirmed_by` (`domain/match_report_state.rs:87-90`).
Aucun changement visible : toutes les cartes suivantes lisent cette valeur.

## Conception

**Domaine**

```rust
pub enum EntryMode { SoloEntry }   // « Je saisis tout » ; E22 ajoute l'autre

MatchReportDomainEvent::EntryModeChosen { mode: EntryMode, started_by: CoachId }
```

- `MatchReportDraft::confirm_selection(confirmed_by, mode)` rend l'avant-match
  et **deux** événements, `SelectionConfirmed` puis `EntryModeChosen`.
- `entry_mode` et `started_by` dans `PreMatch`, `ReadyToPublish`, `Published`,
  propagés comme `created_by` (`from_draft`, `from_pre_match`,
  `into_pre_match`, `from_ready_to_publish`, `unpublish`).
- Rejeu : `(Draft, SelectionConfirmed { confirmed_by })` pose
  `started_by = confirmed_by`, `entry_mode = SoloEntry` ;
  `(PreMatch, EntryModeChosen)` les remplace. `EntryModeChosen` ailleurs :
  `InvalidEventSequence`.

**Les trois chemins de confirmation** appendent les deux événements par
`append_many`, dans une transaction : `create_match_report_use_case::confirm_draft`
(hors calendrier et `confirmer_si_brouillon`) et `update_match_selection_use_case`.
Le mode vient de la commande ; en attendant 632, il vaut `SoloEntry`. Le domain
event émis sur le bus reste `SelectionConfirmed` — `MatchReportConfirmed` est
inchangé.

`CreateMatchReportError` reçoit `Conflict` (carte 623, qui ne l'avait posé que
sur la sélection) : deux « Commencer » simultanés donnent 409 et non 500.

**Projection** : `match_report_proj.entry_mode TEXT NOT NULL DEFAULT 'SoloEntry'`
et `started_by TEXT`. `SelectionConfirmed` y écrit `confirmed_by`,
`EntryModeChosen` le remplace — le même pli que l'agrégat, donc une projection
reconstruite donne le même résultat que la migration. Une méthode de dépôt les
lit, à côté de `find_access_scope`, pour la garde de 634.

**Publisher** : `aiguiller` a un joker (`io/app_events/app_event_publisher.rs:150`).
Les variantes ignorées y sont nommées une à une, `EntryModeChosen` compris :
un événement ajouté casse alors la compilation à cet endroit.

## Données de production

Un événement **ajouté**, l'event store n'est pas réécrit. Migration SQL :

```sql
ALTER TABLE match_report_proj
    ADD COLUMN entry_mode TEXT NOT NULL DEFAULT 'SoloEntry',
    ADD COLUMN started_by TEXT;
UPDATE match_report_proj p
   SET started_by = e.payload->>'confirmed_by'
  FROM match_report_event_store e
 WHERE e.match_report_id = p.match_report_id
   AND e.event_type = 'SelectionConfirmed';
```

Tout compte rendu confirmé passe en « Je saisis tout », démarré par celui qui a
confirmé. Un brouillon garde `started_by` vide. **Mesure sur un dump avant
livraison** : aucun rapport en `PreMatch`, `ReadyToPublish` ou `Published` sans
`SelectionConfirmed` — sinon il resterait sans démarreur.

## Checklist

- [x] `EntryMode`, `EntryModeChosen`, `confirm_selection` à deux événements
- [x] `entry_mode` et `started_by` dans les trois états confirmés, et leur rejeu
- [x] Les trois chemins de confirmation par `append_many` ; `Conflict` dans `CreateMatchReportError`
- [x] Migration : colonnes et remplissage ; projection de `SelectionConfirmed` et `EntryModeChosen` ; lecture pour la garde
- [x] Aiguillage du publisher sans joker
- [x] Tests unitaires : rejeu d'un flux ancien (`SoloEntry`, `confirmed_by`), d'un flux nouveau, à travers publication et dépublication ; `EntryModeChosen` hors avant-match refusé ; conflit relayé
- [x] Mesure sur dump consignée dans la carte
- [x] `make lint`, `make check-arch`, `make test`
