# Récapitulatif de match — afficher les vrais SPP

**Priorité : moyenne** — un affichage faux, sans effet sur le jeu
**Dépend de :** 275 (le barème)
**Fichiers :** `src/app/spp_calculator/domain/calculator.rs`,
`src/infrastructure/match_report/spp_calculator_adapter.rs`,
`src/app/match_report/ports.rs`, `tests/e2e/test_match_report_recap.py`

## Problème

La carte « Performances » du récapitulatif affiche **10 SPP pour tout le
monde**, quel que soit ce que le joueur a fait.

Ce n'est pas un câblage manquant mais un **stub assumé**, qui annonce lui-même
cette carte :

```rust
/// STUB — retourne une valeur plausible (10 SPP) par acteur distinct ayant au
/// moins une action non subie. La vraie règle de calcul (quelles actions
/// donnent combien de SPP, sélection de ruleset Normal/Brutal) est hors scope
/// de cette carte — carte dédiée future.
const STUB_SPP: u8 = 10;
```

C'est un chemin **distinct** de celui qui crédite réellement les joueurs. Les
SPP inscrits dans `players` sont calculés par `player_match_impact_listener`
depuis le barème du corpus ; le récapitulatif, lui, appelle
`spp_calculator::calculate`, qui ne regarde ni le type d'action ni le roster.

Deux chemins, donc deux bugs — et celui-ci ne se voit qu'à l'écran.

## Pourquoi un calcul et non une lecture

Le récapitulatif s'affiche **avant publication** : à ce moment, rien n'a encore
été crédité à personne. Lire les SPP dans `players` donnerait zéro. C'est la
raison d'être du calculateur, et elle reste valable.

Conséquence à tenir : les deux chemins doivent donner le **même** résultat,
puisqu'ils décrivent le même match. Le barème de la carte 275 est le seul
endroit où il doit être écrit.

## Action

Remplacer le stub par la somme réelle, par acteur, des SPP de chaque action —
en consommant le barème de la carte 275.

`ISppCalculatorPort::calculate_match_spp` reçoit déjà `home_roster_id` et
`away_roster_id`, aujourd'hui préfixés d'un underscore et ignorés. Le port a été
conçu pour cette sélection dès l'origine ; il n'y a qu'à s'en servir.

Ce que le stub fait déjà bien et qu'il ne faut pas perdre : une action **subie**
— le joueur blessé — ne crédite rien à sa victime. Un test le couvre déjà.

## Tests

**Unitaires** — le cumul par acteur sur plusieurs actions, la différence entre
types d'action, l'exclusion des blessures subies, et le même match rendant des
totaux différents selon le barème du roster. Ce dernier est celui qui distingue
une vraie correction d'un barème unique recodé en dur.

**E2E** — la page de récapitulatif, où **aucun test ne vérifie aujourd'hui la
valeur affichée** : `test_match_report_recap` contrôle que la carte est présente
ou absente, jamais ce qu'elle contient. C'est exactement ce qui a laissé vivre
le 10 pendant toute la vie du stub.

Le test doit être **discriminant** : construire un match où le total juste
diffère à la fois de 10 et du nombre d'actions, sans quoi il pourrait passer sur
une implémentation fausse.

## Checklist

- [ ] `STUB_SPP` supprimé, aucune constante de barème dans `spp_calculator`
- [ ] Le barème vient de la carte 275, il n'est pas réécrit ici
- [ ] `home_roster_id` / `away_roster_id` réellement utilisés
- [ ] Une action subie ne crédite toujours rien
- [ ] Test unitaire : deux barèmes, deux totaux, sur le même match
- [ ] E2E : la valeur affichée est vérifiée, et ne peut pas valoir 10 par hasard
- [ ] `make check-arch` au vert, `make test` au vert
