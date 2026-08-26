# Lancer le dé par requête

**Priorité : haute**
**Dépend de :** 408 — et de `ITeamAccessPort`, cf. plus bas
**Conception :** `docs/specs/erreurs-couteuses/ecran-du-jet/{04-dtos,05-use-cases,07-integration}.md`
**Fichiers :** `src/app/teams/ports.rs`,
`src/infrastructure/teams/dice_adapter.rs` (nouveau),
`use_cases/apply_costly_mistakes_use_case.rs` (nouveau),
`io/web/costly_mistakes.rs` (nouveau), `routes.rs`, `context.rs`, `src/main.rs`

## Objectif

Le jet s'effectue et s'applique par une requête. **Sans écran** : la carte 410
lui en donne un.

```
POST /app/{space_id}/teams/{team_id}/costly-mistakes/roll
```

**Le POST n'a pas de corps.** L'équipe est dans le chemin, le coach dans la
session. Rien n'entre, donc rien n'est à valider — et **le client ne peut pas
proposer de jet**.

## Le hasard passe par un port

```rust
pub trait IDiceRoller: Send + Sync {
    fn d6(&self) -> u8;
    fn d3(&self) -> u8;
    fn two_d6(&self) -> (u8, u8);
}
```

Le précédent du projet tire en dur dans le use case (`random_draw.rs:44`), et ses
tests ne portent que sur la répartition, jamais sur le tirage. **Ça ne suffit pas
ici** : il faut prouver qu'à 345 kPo, un 1 donne un incident majeur et retire
exactement 170. Sans jet forçable, ce test n'existe pas et la table de la carte
408 reste non vérifiée de bout en bout.

`two_d6` rend un **couple**, pas deux appels à `d6` : les deux dés d'une
catastrophe sont un seul geste, et un test qui enchaîne deux dés truqués devient
vite illisible.

## L'orchestration ne décide de rien

```
1. charger l'équipe                                → TeamNotFound
2. roll = dice.d6()
3. incident = incident_for(team.treasury, roll)             ← domaine
4. selon incident.dice_needed() : rien / d3() / two_d6()    ← domaine
5. team.apply_costly_mistakes(roll, damage_dice)?           → Domain
6. repo.append(…)                                  → Repository
7. info!(roll, ?damage_dice, ?incident, gp_lost, …)
```

Le use case tire ce que le domaine lui demande, dans l'ordre que le domaine
impose. **Le dé est tiré avant la vérification de phase** : un second POST tirera
donc un dé inutile avant d'être refusé — sans conséquence, il n'est écrit nulle
part.

Le `info!` est **sur cible `kreek::`**, sinon il n'existe pas en production. Une
contestation doit être vérifiable sans ouvrir l'event store.

## Les quatre échecs

| Cas | Code |
|---|---|
| pas de session | 401 |
| équipe introuvable | 404 |
| ni propriétaire ni administrateur | 403 |
| mauvaise phase — **second jet** | **409** |

**409 et non 422** : la requête est bien formée, c'est l'état qui a changé.
`edit_match_report` répond déjà ainsi sur un rapport publié.

L'idempotence ne demande **ni verrou ni jeton** : `CostlyMistakesApplied` repose
`ReadyToPlay`, donc `expect_phase(CostlyMistakes)` refuse le second jet. Elle
sort du modèle.

## `ITeamAccessPort` — à ne pas dupliquer

Le droit est celui de la carte **389** : coach propriétaire, administrateur
d'espace, administrateur de compétition. **Si la 389 est livrée avant, le port
existe et cette carte s'en sert ; sinon, elle le crée et la 389 s'en servira.**

Le contrôle vit dans la couche web, comme le précédent du projet —
`post_update_roster` appelle `can_spend_spp` avant d'entrer dans le use case.

Il garde le **POST**, pas seulement l'affichage : l'URL est devinable, et un jet
a un effet financier.

## Checklist

- [ ] `IDiceRoller` dans `ports.rs`, `dice_adapter.rs` sur `rand`, câblé dans
      `main.rs` et `TeamsContext`
- [ ] `apply_costly_mistakes_use_case`, instrumenté, avec son `info!` de résultat
- [ ] `CostlyMistakesOutcome` portant le jet, les dés, l'incident, la perte et
      les deux soldes
- [ ] Route `COSTLY_MISTAKES_ROLL` et son handler, sans extracteur de corps
- [ ] Le droit vérifié avant le use case
- [ ] `post_validate_dismissals_phase` redirige selon l'issue de la 408
- [ ] Tests unitaires, sur un dé truqué :
  - [ ] 345 kPo + jet 1 → majeur, perte 170, un seul événement
  - [ ] 345 kPo + jet 5 → crise évitée, perte nulle, événement quand même émis
  - [ ] 560 kPo + jet 1 + (3,4) → catastrophe, perte 490
  - [ ] second POST → 409, **aucun second événement**, trésorerie inchangée
  - [ ] coach tiers → 403, aucun événement
  - [ ] hors phase → 409
- [ ] `make lint`, `make check-arch`, `make test`
