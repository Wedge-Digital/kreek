# E13 — Gestion des erreurs coûteuses

**État :** `ready` — 4 cartes, 0 faite. Spécifiée par le workflow feature les
2026-08-25 et 26.
**Conception :** `docs/specs/erreurs-couteuses/`

## La fonction

Une équipe qui garde plus de **100 kPo** après ses recrutements et ses renvois
est exposée aux ennuis : un jet décide de ce qu'il lui en reste. C'est une
**nouvelle phase du cycle de vie de l'équipe**, la dernière avant qu'elle
redevienne prête à jouer.

```
Recruitment → Dismissals → CostlyMistakes → ReadyToPlay
                        └──────────────────────↑  trésorerie < 100 kPo
```

Sans elle, une trésorerie s'accumule sans risque, et rien ne pousse un coach à
dépenser. C'est le contrepoids que le règlement prévoit, et le seul du jeu qui
sanctionne l'inaction.

## État

Rien n'est implémenté, mais **tout l'aval l'est déjà**, écrit avant son
producteur : `CostlyMistakesApplied`, `IncidentType`, le débit écrêté au solde,
la ligne du grand livre au motif `CostlyMistake`, le retour en `ReadyToPlay`, et
les deux listeners qui recalculent la valeur d'équipe et purgent les paniers.

**Personne ne produit cet événement.** C'est ce que cette épic ajoute.

## Les cartes

| # | Carte | Apport |
|---|---|---|
| 408 | Le domaine des erreurs coûteuses | la table, les effets, la phase, les deux sorties de la validation des renvois — 36 tests |
| 409 | Lancer le dé par requête | port du dé, use case, POST gardé par le droit — sans écran |
| 410 | L'écran du jet | page, fragment, animation à durée plancher, CSS, bandeau |
| 411 | Les erreurs coûteuses sous Playwright | six scénarios |

## Ce qui commande l'ordre

**Strictement séquentiel** : `408 → 409 → 410 → 411`. Aucun parallélisme, tout
étant dans `teams` et tout s'empilant — contrairement à la Haine, dont l'écran et
la traversée vers `players` ne se touchaient pas.

La **409 a besoin de `ITeamAccessPort`**, celui de la carte 389 : la première des
deux livrées le crée, l'autre s'en sert.

## Ce que l'épic ne couvre pas

- **La consultation du jet après coup**, écartée en phase 2 : elle demandait un
  champ dérivé, un second rendu du contrôleur et un CTA conditionnel pour une
  page qu'on regarde une fois. Un coach qui recharge après le jet ne reverra pas
  son résultat.
- **L'onglet Trésorerie** (carte 48). La ligne du mouvement existe déjà au grand
  livre ; il manque l'écran qui la montre. C'est lui qui rendra le montant
  consultable pour toujours, au bon endroit — celui où l'on va chercher où est
  passé l'argent.
- **La retraite temporaire** (carte 39), toujours hors du chemin des phases.

## Terminé quand

Une équipe qui valide ses renvois avec **150 kPo en caisse** arrive sur l'écran
du jet, lance le dé, voit son résultat détaillé, et retrouve sur sa fiche une
trésorerie diminuée du montant annoncé — pendant qu'une équipe à **99 kPo** passe
directement en « prête à jouer » sans rien voir.

## Ce que son histoire a appris

Cette épic naît de la **carte 40**, écrite en 2026 et classée `done` le
2026-08-18 par le commit `2bd45c3`, qui clôturait l'épic E01 « par vérification
une par une dans le code ». La vérification avait vu les types et l'événement —
qui existaient bel et bien — sans voir que **personne ne les produisait**. Sa
checklist n'a jamais eu une seule case cochée.

Elle est revenue en `to_be_refined` le 2026-08-25, quand une question sur les
erreurs coûteuses a fait regarder le code de plus près. Ses effets étaient
marqués « à définir » depuis le premier jour : la source consultée à l'époque ne
les donnait pas.

**Deux leçons y sont attachées.** Une carte dont la checklist est vierge n'est
pas terminée, quoi qu'en dise son dossier. Et un critère d'épic qui affirme
qu'une équipe « paie ses erreurs coûteuses » — celui de E01 — doit être constaté,
pas déduit de l'existence d'un enum.
