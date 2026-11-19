# L'étape 2 annonce moins de journaliers qu'elle n'en ajoutera

**Priorité : haute** — l'écran contredit ce que le rapport va faire
**Contexte :** `teams` (le widget) · **Sans épic** · **Dette de la carte 488**

## Le constat

Le bloc « Journaliers » de l'étape 2 calcule `11 - player_count`, et
`player_count` vient de `team_match_context_widget.rs:40` :

```rust
let player_count = state.teams.squad_port.find_squad(&query.team_id).await.len();
```

`find_squad` rend l'effectif **entier** — c'est son contrat, et il est
délibéré : « la valeur d'équipe ne somme que les disponibles quand les quotas
comptent tous les occupants, un port qui filtrerait à la source servirait l'un
et trahirait l'autre » (`teams/ports.rs:76`). Prendre `.len()` revient donc à
compter tout le monde.

**La règle réelle, elle, compte les alignables.** C'est
`count_available_by_team_id` que `match_report` interroge pour décider des
journaliers :

```sql
WHERE team_id = $1 AND membership = 'Active' AND participation_status = 'Available'
```

L'écran annonce donc moins de journaliers que le rapport n'en ajoutera, pour
**tout** joueur indisponible.

## Ce que la carte 488 a changé, et ce qu'elle a laissé

Le défaut préexiste : un blessé était déjà compté comme présent. La 488 l'a
aggravé sans le voir — elle a décidé que **le mort appelle un journalier**, et a
donné au domaine `Squad::size()` qui écarte les perdus, mais n'a pas repris ce
consommateur-ci, qui n'est pas passé par `Squad`. L'écran dit donc aujourd'hui
l'inverse d'une décision prise explicitement.

## La correction

Compter les alignables, via le prédicat du domaine plutôt qu'une comparaison de
chaîne :

```rust
membres.iter().filter(|m| m.presence.alignable()).count()
```

`SquadPresence::alignable()` est ce que `teams` sait déjà dire de sa présence
depuis la 488 — l'ACL a traduit, le domaine décide, et la vue applique. Aucune
requête de plus : l'effectif est déjà chargé.

**Le champ est renommé `available_player_count`.** `player_count` disait
« l'effectif », et c'était bien ce qu'il rendait ; il rend désormais autre
chose. Garder le nom aurait laissé le prochain lecteur croire au premier sens.
Le nom rejoint celui de la règle réelle, `count_available_by_team_id`.

Trois usages, tous dans `pre-match.html` : le calcul des journaliers et deux
libellés (« X joueurs, aucun journalier nécessaire »). Aucun autre consommateur
du JSON.

## Ce que la carte ne fait pas

**Elle ne touche pas au calcul de la valeur d'équipe.** `team_value.rs` compte
déjà les alignables et ajoute ses journaliers correctement — c'est cet écran-là
qui divergeait, pas la TV.

**Elle ne déplace pas le calcul des journaliers dans le domaine.** Il vit
aujourd'hui dans le JS de la page (`11 - player_count`) et le nombre 11 y est en
dur, alors que le domaine porte `MATCH_SQUAD_SIZE`. Défaut réel, autre sujet.

## Tests

Le comptage sort dans une fonction pure, éprouvée sur un effectif mêlant les
trois présences — un alignable, un empêché, un perdu. Le handler lui-même n'est
pas testable unitairement : il prend un `AppState`, dont le projet n'a aucun
constructeur de test.

## Checklist

- [x] `player_count` → `available_player_count`, compté par `alignable()`
- [x] Les usages du gabarit suivent — **cinq et non trois** : le calcul des
      journaliers en cite deux, et chacun des deux libellés existe côté domicile
      et côté visiteur
- [x] Quatre tests unitaires du comptage — les trois présences mêlées, les deux
      indisponibilités traitées pareil, un effectif plein, un effectif vide.
      **Falsifié** : rétablir `.len()` fait échouer
      `seuls_les_alignables_sont_comptes`
- [x] `make lint`, `make check-arch` (17 axes), `make test` (1639),
      `make e2e` (**354 passés**, suite complète 67/67, 0 échec)

## Terminé quand

Une équipe de onze joueurs dont un blessé annonce « 10 joueurs, 1 journalier
ajouté » à l'étape 2 — le même nombre que celui que le rapport ajoutera.
