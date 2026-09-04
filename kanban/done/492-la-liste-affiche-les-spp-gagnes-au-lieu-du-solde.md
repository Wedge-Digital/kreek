# La colonne SPP de la liste affiche les gains, pas le solde

**Priorité : haute** — deux écrans donnent deux chiffres pour la même chose
**Contexte :** `players` · **Sans épic** · **Demandée par :** l'utilisateur

## Le constat

Dans le tableau de la feuille d'équipe, la colonne SPP affiche le **cumul des
points gagnés** depuis toujours. Acheter une compétence ou une caractéristique
n'en retire rien. Un joueur qui a gagné 20 SPP et tout dépensé y affiche encore
20, alors qu'il ne peut plus rien acheter.

**La fiche du joueur, elle, dit juste.** `player_detail_controller.rs:358` appelle
`spp_remaining()` et affiche gagné / dépensé / réserve. Les deux écrans se
contredisent donc sur la même donnée.

## La cause

`players_proj.spp` n'est pas un solde. Quatre écritures le touchent :

| `player_repository.rs` | événement | effet |
|---|---|---|
| 276 | `TouchdownScored`, `CasualtyInflicted`, `MatchMvpNamed` | `spp = spp + gain` |
| 595 | `PlayerSppCustomised` | `spp = spp + montant` |
| 697 | `UndoEffect::Spp` | `spp = GREATEST(spp - montant, 0)` |
| 409 | `MatchImpactReverted` | réécrit depuis l'agrégat rejoué |

Les deux soustractions sont des chemins d'**annulation**.
`PlayerSkillPurchased` et `PlayerStatIncreased` écrivent bien dans la projection
— `value_kpo`, `acquired_skills` — mais **jamais `spp`**. La colonne ne peut donc
que monter.

Le domaine, lui, sait compter (`player.rs:898`) :

```rust
/// SPP encore disponibles — dérivé, jamais stocké (cohérent avec l'event sourcing).
pub fn spp_remaining(&self) -> u32
```

## Ce qu'on ne fait pas, et pourquoi

**On n'ajoute pas de colonne à la projection.** Le commentaire du domaine est
explicite : la réserve est dérivée, jamais stockée. La stocker demanderait une
migration, deux écritures de plus sur des événements qui n'y touchent pas
aujourd'hui, et une reprise des lignes existantes — pour contredire une décision
prise.

**Et ce serait payer une requête qu'on a déjà.** `resolve_team_stats` hydrate
**déjà** tous les agrégats de l'équipe depuis l'event store, en une seule
requête, parce que la projection ne porte ni les malus de séquelles ni les
augmentations achetées. La réserve est exactement dans ce cas : la même passe la
calcule sans un aller-retour de plus.

## Un couplage à ne pas reproduire

`resolve_team_stats` filtre aujourd'hui par `filter_map` : un joueur dont le
poste est introuvable au catalogue disparaît de la carte des caractéristiques, et
la table affiche des tirets. **La réserve ne doit pas hériter de ce sort** — elle
ne dépend pas du catalogue. Keyer les deux sur la même carte ferait perdre ses
SPP à un joueur dont seul le poste est illisible.

D'où une carte unique portant les deux, la résolution des caractéristiques
restant `Option` à l'intérieur.

## Le repli

Si l'agrégat manque — joueur introuvable au rejeu — la cellule affiche un tiret,
comme le maillot absent le fait déjà. **Pas le cumul** : retomber silencieusement
sur le chiffre faux qu'on corrige serait pire que ne rien afficher.

## Décisions prises

- **L'en-tête reste « SPP ».** Pas de renommage.
- **Le tri n'est pas vérifié** — hors périmètre, l'utilisateur l'a écarté.

## Ce que la carte ne fait pas

**Elle ne touche pas la projection.** `players_proj.spp` reste le cumul des
gains, et c'est ce que veulent ses autres lecteurs.

**Elle ne touche pas les autres écrans.** La fiche du joueur, le panneau de
dépense et le journal d'évolution appellent déjà `spp_remaining()` ; ils sont
justes.

**Elle n'ajoute pas de survol détaillé.** Afficher « 6 disponibles sur 20
gagnés » au survol a été évoqué et écarté : la colonne du nom déborde déjà en
mobile (carte 489), et un `title` de plus se discute à part.

## Checklist

- [x] `resolve_team_stats` devient `resolve_team_derived` et rend caractéristiques
      **et** réserve, en une passe — via `PlayerDerived`, dont seul le champ
      `stats` est `Option` : la réserve ne dépend pas du catalogue
- [x] `PlayerRowVm.spp` passe à `Option<u32>`, alimenté par `spp_remaining()`
- [x] Le gabarit rend un tiret quand la valeur manque
- [x] Test e2e `test_la_colonne_spp_de_la_liste_est_le_solde_pas_le_cumul`, avec
      un joueur dédié dans la fixture (la réserve n'est pas partagée). Il vérifie
      l'égalité liste/fiche **avant** l'achat, la baisse après, et l'égalité de
      nouveau — la baisse seule passerait si les deux écrans divergeaient d'une
      constante
- [x] **Falsifié** : l'ancien comportement rétabli, le test échoue sur
      « la colonne n'a pas baissé après l'achat : 8 puis 8 »
- [x] Pas d'entrée à ajouter : le test rejoint `test_player_spp_spending`, déjà
      dans la carte d'impact
- [x] `make lint`, `make check-arch` (17 axes), `make test` (1632)

## Terminé quand

Sur la base de démonstration : un joueur à qui l'on achète une compétence voit
la colonne SPP de la feuille d'équipe baisser du coût payé, et ce chiffre est le
même que la réserve affichée sur sa fiche.
