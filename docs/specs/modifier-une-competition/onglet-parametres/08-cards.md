# Onglet Paramètres · Phase 8 : cartes kanban

**Épic :** `kanban/epics/ready_to_be_done/E14-modifier-une-competition-en-cours.md`
**Phases 2 à 7** : ce dossier

Dix cartes, en `kanban/ready_to_be_done/`, en trois vagues plus les tests.

## Vague 1 — le socle, sans écran

Les deux cartes où le risque est réel. Aucune ne touche un écran, aucune ne
dépend de l'autre, et chacune est livrable seule.

| N° | Carte | Ce qu'elle pose |
|---|---|---|
| 417 | Les réglages deviennent gardés par le domaine | `RankingGroupConfig` encapsulé, trois méthodes sur `CompetitionRules`, cinq variantes de `DomainError`, 14 tests |
| 418 | `ranking` sait se rejouer | `stats_between`, deux méthodes de dépôt, `recompute_season_ranking_use_case` |

## Vague 2 — la place de l'onglet

| N° | Carte | Dépend de |
|---|---|---|
| 419 | Le tableau de bord et les résultats quittent l'administration | rien |
| 420 | L'onglet Paramètres, coquille vide | 419 |

La démolition est séparée de la fondation : la 419 est autonome, se relit seule,
et n'a aucune raison d'attendre le reste.

## Vague 3 — un panneau par carte

| N° | Panneau | Ce qui lui est propre | Dépend de |
|---|---|---|---|
| 421 | Informations générales | deux écritures, un seul use case · l'erreur sous le champ | 420 |
| 422 | Points de classement | **le port de commande, l'adapter, l'injection** · le recalcul synchrone | 418, 420 |
| 423 | Poules | `save_structure_and_prune_groups` · le compteur d'affectations défaites | 417, 420 |
| 424 | Tiers & coups de pouce | la collecte JS de l'événement du picker | 417, 420 |
| 425 | Visibilité | la plus simple · trois champs à préserver | 420 |

## Vague 4

| N° | Carte | Dépend de |
|---|---|---|
| 426 | Les tests E2E de l'onglet | 421 à 425 |

## Trois choix de découpage, et leur raison

**Le port de recalcul est dans la 422, pas dans la 418.** La 418 livre
« `ranking` sait rejouer une saison » — un use case appelable et prouvé. Le port
et son adapter n'existent que pour qu'un POST de `competitions` le déclenche :
ils appartiennent au panneau qui s'en sert. Les isoler donnerait une carte de
trois fichiers sans rien à démontrer.

**Les tests E2E forment une carte, pas une ligne de checklist dans chacune.**
Les scénarios qui comptent traversent plusieurs panneaux — le recalcul part du
barème et se vérifie dans le classement, la cascade part des poules et se
vérifie dans l'onglet Poules. Répartis, ils seraient écrits cinq fois à moitié.

**Les cartes 415 et 416 restent hors de cette suite.** Le plafond de
participants et les treize routes de mutation sans contrôle d'accès sont des
défauts trouvés en instruisant les phases 4 et 7. Ils se prennent quand on veut,
dans n'importe quel ordre, et les mêler à cette épic ferait passer une
correction de sécurité pour une étape de fonctionnalité.

## Ce que la phase 8 clôt

Le workflow s'arrête ici. L'implémentation se fait carte par carte, sous les
règles ordinaires du `CLAUDE.md` — rappel de la carte, plan de réalisation,
validation avant de coder.
