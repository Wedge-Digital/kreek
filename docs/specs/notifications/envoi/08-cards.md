# Phase 8 — Cartes : les deux unités

Dix cartes. `configuration/` en produit quatre, `envoi/` six.

| # | Carte | Unité | Dépend de |
|---|---|---|---|
| 331 | Réglages : domaine et persistance | configuration | — |
| 332 | Widget de réglage et son hôte admin | configuration | 331 |
| 333 | Widget de réglage dans le magicien | configuration | 331, 332 |
| 334 | Retrait des trois réglages morts | configuration | 333 |
| 335 | Journal d'envois : table et repository | envoi | — |
| 336 | Domaine de l'ordonnancement — `due_today()` | envoi | 331 |
| 337 | Résolution des destinataires | envoi | — |
| 338 | Les quatre gabarits d'email | envoi | 337 |
| 339 | Le cœur d'expédition | envoi | 335, 337, 338 |
| 340 | Les deux déclencheurs — CLI et listener | envoi | 336, 339 |

## La propriété recherchée

**Aucune carte avant la 340 ne fait partir un email.** La livraison est
l'ensemble, pas une carte. C'est ce qui empêche de mettre en production un écran
qui promet des emails que rien n'envoie.

## Chemin critique

331 est la **seule dépendance croisée** entre les deux unités — 336 a besoin de
`CompetitionNotifications`. Après elle, les deux chaînes avancent en parallèle :

```
331 ─┬─ 332 ── 333 ── 334                          (configuration)
     └─ 336 ─────────────────┐
335 ─────────────────────┐   │
337 ─┬───────────────────┤   │
     └─ 338 ─────────────┴─ 339 ── 340             (envoi)
```

335 et 337 ne dépendent de rien : ce sont les deux points d'entrée si l'on veut
paralléliser à deux personnes.

## Trois choix de découpage, et leur justification

**335 et 336 restent séparées** bien qu'elles se suivent. L'une est de la
persistance, l'autre du domaine pur testable sans base. Les fusionner ferait une
carte mêlant un index PostgreSQL subtil et un calcul de dates — deux façons de se
tromper qui n'ont rien à voir.

**338 est une carte à elle seule.** Quatre emails HTML avec leurs contraintes de
client de messagerie — logo en URL absolue, dimensions en attributs, aucune
feuille externe — plus une vérification visuelle que rien n'automatise. Ce n'est
pas un détail de 339.

**340 fusionne les deux déclencheurs**, décision prise à la validation de cette
phase. Séparés, chacun tenait en une session ; ensemble, c'est la plus grosse
carte du lot, mais livrer l'un sans l'autre laisserait la fonctionnalité à moitié
active — la moitié la plus difficile à diagnostiquer, puisque certains emails
partiraient et d'autres non.

## Ce qu'aucune carte ne couvre

Le **rendu visuel des quatre emails dans un vrai client de messagerie**. C'est une
case de la carte 338, à faire à la main, comme les maquettes de la phase 1 l'ont
été. Aucun test du dépôt ne le voit.
