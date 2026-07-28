# `competitions` — Le garde-fou s'applique aux suppressions en masse

**Priorité : moyenne**
**Dépend de :** `239-sp-garde-fou-pairing-publie.md`
**Fichiers :** `src/app/competitions/io/web/admin/schedule_actions.rs`, `src/app/competitions/use_cases/admin/delete_pairing_use_case.rs`, `src/app/competitions/io/web/templates/admin/schedule.html`

## Objectif

Trois autres chemins suppriment des pairings et émettent `PairingDeleted` :

| Action | Handler |
|---|---|
| Supprimer une journée | `delete_round` |
| Vider les matchs d'une journée | `post_clear_round_pairings` |
| Vider toute la saison | `post_clear_all` |

Sans cette carte, le garde-fou de la 239 se contourne en un clic : « Vider les
matchs » efface aussi les rencontres à rapport publié.

## Conception

### Partiel avec compte-rendu, pas refus global

Un refus global sur « vider toute la saison » dès qu'un seul match est publié
rendrait l'action inutilisable en cours de saison — c'est justement à ce
moment-là qu'on veut regénérer les journées restantes. On supprime donc **tout
ce qui est supprimable** et on rend compte du reste, dans l'esprit des
`skipped_teams` / `skipped_rounds` / `skipped_groups` déjà en place.

```rust
#[derive(Serialize, Default)]
struct ScheduleActionResult {
    …
    #[serde(skip_serializing_if = "Vec::is_empty")]
    skipped_matches: Vec<String>,   // « Les A – Les B (J3) »
}
```

Message côté `handleScheduleActionResponse` :

> Rencontre(s) conservée(s), rapport déjà publié : Les A – Les B (J3), …

### Le use case fait le tri

`delete_pairing_use_case` gagne une entrée « lot » : une seule consultation du
port pour toute la liste (d'où la lecture batch de la 239), partition entre
supprimables et conservés, suppression des premiers, événements émis pour eux
seuls, retour de la liste des seconds.

Attention à `delete_round` : la journée elle-même ne peut pas être supprimée
s'il reste des pairings conservés — la supprimer ferait cascader la suppression
en base sur `competition_match_day_pairings`, contournant le garde. Dans ce cas
on supprime les pairings supprimables, **on garde la journée**, et le
compte-rendu l'explique.

## Checklist

- [ ] Entrée « lot » du use case : une consultation du port, partition, événements pour les seuls supprimés
- [ ] `delete_round` : journée conservée s'il reste au moins un match publié
- [ ] `post_clear_round_pairings` et `post_clear_all` : partiel + compte-rendu
- [ ] Champ `skipped_matches` dans `ScheduleActionResult`
- [ ] Message dédié dans `handleScheduleActionResponse`
- [ ] Test : lot mixte → seuls les supprimables partent, les autres sont rendus dans le compte-rendu
- [ ] Test : lot entièrement publié → aucune suppression, aucun événement
- [ ] Test : `delete_round` conserve la journée quand un match est conservé
- [ ] Test E2E : vider une journée contenant un match publié → le match reste, le message s'affiche
- [ ] `make test` passe
- [ ] `make check-arch` passe
