# Les erreurs coûteuses — Progression

Une équipe qui garde plus de **100 kPo** après ses recrutements et ses renvois se
soumet à un jet : elle peut y perdre une partie de sa trésorerie, voire presque
tout. C'est une **nouvelle phase du cycle de vie de l'équipe**, la dernière avant
qu'elle redevienne prête à jouer.

## Maquette (Phase 1 ✅)

`assets/rawpages/html/app-team-costly-mistakes.html` — validée et commitée
(`6a27e8b`).

## Progression

| Page | Front | Back | DTOs | Use cases | Domaine | Intégration | Cartes |
|---|---|---|---|---|---|---|---|
| L'écran du jet | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | |

## La séquence

```
Recruitment → Dismissals → CostlyMistakes → ReadyToPlay
                        └──────────────────────↑  trésorerie < 100 kPo
```

Le déclencheur est le bouton **« Valider les renvois »**. La retraite temporaire
(carte 39) reste hors du chemin, comme aujourd'hui.

## La table de déclenchement

| Trésorerie | Crise évitée | Incident mineur | Incident majeur | Catastrophe |
|---|---|---|---|---|
| 100 – 199 kPo | 2–6 | 1 | — | — |
| 200 – 299 kPo | 3–6 | 1–2 | — | — |
| 300 – 399 kPo | 4–6 | 2–3 | 1 | — |
| 400 – 499 kPo | 5–6 | 3–4 | 1–2 | — |
| 500 – 599 kPo | 6 | 4–5 | 2–3 | 1 |
| 600 kPo et + | — | 5–6 | 3–4 | 1–2 |

**Les tranches sont fermées à la centaine**, et non à 195, 295, 395 comme le
règlement les écrit. Celui-ci suppose des montants en multiples de 5 kPo ; la
trésorerie est un entier de kPo, et une équipe à 197 ne doit tomber dans aucun
trou. Aucun montant régulier ne change de tranche pour autant.

## Les effets

| Incident | Effet |
|---|---|
| Crise évitée | rien |
| Incident mineur | perte de **1D3 × 10 kPo** |
| Incident majeur | perte de **la moitié de la trésorerie, arrondie au 5 kPo inférieur** |
| Catastrophe | perte de **tout, sauf 2D6 × 10 kPo** |

L'arrondi porte sur **la perte**, pas sur ce qui reste : à 345 kPo, un incident
majeur retire 170 et en laisse 175.

## Décisions déjà prises

**Le système tire, le coach clique.** Partout ailleurs — Facteur Fans, blessures
— le coach lance son dé physique et saisit le résultat. Ici il n'en a pas sous la
main : la phase se joue seul devant un écran, entre deux matchs. **C'est une
exception assumée**, pas un oubli : qu'on ne vienne pas « corriger »
l'incohérence dans six mois.

**Le jet a lieu au clic, une seule fois.** Pas à l'affichage : un rechargement
en produirait un autre, et le coach ferait défiler les résultats jusqu'au bon.

**Le résultat n'est pas consultable après coup.** Un champ dérivé, un second
rendu du contrôleur et un CTA conditionnel pour une page qu'on regarde une fois :
le jeu n'en valait pas la chandelle. Un coach qui recharge après le jet ne reverra
pas son résultat — le montant, lui, figure au grand livre avec le motif
`CostlyMistake`, et sera lisible quand l'onglet Trésorerie existera (carte 48).

**L'écran s'atteint par le bandeau d'état**, comme les autres phases, et n'existe
que pendant `CostlyMistakes`.

**Le jet est réservé** au coach propriétaire et aux administrateurs d'espace ou de
compétition — la règle de la carte 389.

**Sous 100 kPo, aucun écran.** L'équipe passe de `Dismissals` à `ReadyToPlay`
sans rien voir.

## Ce qui existe déjà

`TeamDomainEvent::CostlyMistakesApplied { roll, incident, gp_lost }` est défini
depuis longtemps, avec `IncidentType { None, Minor, Major, Catastrophe }` — les
quatre colonnes de la table, au mot près. Son aval est câblé : débit **écrêté au
solde**, ligne au grand livre avec le motif `CostlyMistake`, retour en
`ReadyToPlay`, recalcul de valeur d'équipe et purge des paniers par les
listeners. **Seul l'amont manque** : personne ne produit cet événement.

C'est le constat de la carte 40, ramenée de `done/` le 2026-08-25 après
vérification.
