# Phase 8 — Les cartes : l'encart du coach connecté

**Entrée** : les phases 2 à 7 validées.

**Épic** : E16, qui passe à **26 cartes** et quitte `to_be_refined/` pour
`ready_to_be_done/` — son périmètre est désormais entièrement conçu.

## Les quatre cartes

| N° | Carte | Vague | Dépend de |
|---|---|---|---|
| 529 | L'encart sait quelles campagnes sont ouvertes | 7 — l'encart | 510 |
| 530 | La garde de saison quitte l'administration | 7 | — |
| 531 | L'encart du coach connecté | 7 | 529, 530, 516 |
| 532 | Les tests e2e de l'encart | 7 | 531 |

## Ce qui commande l'ordre

**529 et 530 avant 531** — la requête et la garde. La 530 n'a aucune dépendance
et se livre à tout moment ; elle est même **utile en soi**, puisqu'elle sort une
vérification de périmètre de dessous un contrôle d'administration où elle n'avait
rien à faire.

**531 avant 532**, évidemment.

## Un choix de découpage

La 531 est une grosse carte : deux handlers, un service d'hydratation, trois VM,
un gabarit à deux mises en forme, une feuille, un conteneur dans la page hôte.
Elle n'est pas coupable en deux — l'encart est un écran, et une moitié d'écran ne
se livre pas.

Le découpage tentant aurait été « le service » puis « le widget ». Il aurait
produit une carte dont le livrable est un objet que personne n'appelle, et dont
rien ne prouve qu'il rend ce qu'il faut : c'est le gabarit qui révèle qu'un champ
manque.

## Deux cartes hors épic, écrites plutôt que promises

La conception a trouvé deux défauts qui ne sont pas de ce chantier. Les laisser
en note dans une spec, c'est la façon éprouvée de les perdre — le `CLAUDE.md` en
fait une règle : *rien n'est complet tant qu'il reste une carte*, et une note
n'est pas une carte.

| N° | Carte | Trouvée par |
|---|---|---|
| 533 | Le middleware CSRF décrit n'existe pas | l'investigation de phase 1 |
| 534 | `admin_scope` ne vérifie rien d'administratif | la phase 3 de cette unité |

La 533 ne tranche pas d'elle-même entre corriger la description et écrire le
middleware : elle pose les deux arguments et exige qu'on choisisse. La 534 est un
renommage sans changement de comportement, dont la preuve est que les tests
passent sans être touchés.

## Ce que la conception a produit en tout

| | |
|---|---|
| Unités conçues | 3 |
| Cartes | 26 dans l'épic, 2 hors épic |
| Règles métier | 30, dont **15 apparues après la phase 1** |
| Phases sans objet | 2, expliquées plutôt que laissées vides |

Les quinze règles tardives sont le résultat qui compte : elles n'auraient pas été
trouvées en codant. Et **quatre d'entre elles ont corrigé une phase déjà
validée** — R23 la clôture calculée (phases 3, 4 et 5), R24 les rencontres qui
appartiennent à la journée (phase 6), R27 les trois motifs de fermeture (phase
4), R28 les trois autorisations (phase 6 et trois cartes).
