# Feature — Points bonus de classement

Calcul des points bonus de classement à partir des actions d'un match, en plus
des points V/N/D. Trois bonus, chacun **activable indépendamment** par compétition :
offensif, défensif, agressif (nouveau).

## État des lieux (constat initial)

- Les bonus **offensif** et **défensif** existent déjà côté `competitions` (domaine
  `RankingRules`, saisie formulaire admin, persistance JSONB) **mais ne sont jamais
  calculés** : l'adapter ACL `competition_info_adapter.rs` ne transmet que
  win/draw/lose au BC `ranking`.
- Le bonus **agressif** n'existe pas encore.
- L'app event `MatchReportPublished` transporte **déjà** toutes les actions de
  chaque équipe (`Touchdown`, `Sortie`, …) — aucune donnée à ajouter au payload.

Cette feature = brancher les bonus offensif/défensif jusqu'au calcul + créer le
bonus agressif de bout en bout.

## Règles métier validées (référence partagée)

Les 3 bonus sont **cumulables** entre eux, s'**ajoutent** aux points V/N/D, et sont
**indépendants du résultat** (une équipe qui perd peut les toucher). Évaluation
**par équipe**, sur un match donné. **Un bonus n'est calculé que s'il est activé**
(`activated == true`) pour la compétition — désactivé ⇒ 0 point.

| Bonus | Condition (par équipe) | Config |
|---|---|---|
| Offensif | TD **marqués** ≥ seuil | `activated`, `points` (X), seuil TD marqués |
| Défensif | TD **encaissés** ≤ seuil | `activated`, `points` (X), seuil `max_td_conceded` (défaut 1) |
| Agressif | **Sorties** (`Sortie` seule) infligées à l'adversaire **> Y** (strict) | `activated`, `points` (X), `min_casualties` (Y) |

Précisions :
- « Sortie » = action `MatchActionType::Sortie` uniquement (pas `Blesse`, pas `Agression`).
- Comparateur agressif **strict** (`>`), les autres sont **larges** (`≥` / `≤`).
- Point d'attention (à trancher en phase domaine, unité calcul) : le champ offensif
  s'appelle `diff_td` alors que la sémantique est « TD marqués ≥ seuil » — renommage
  proposé `min_td` avec compat désérialisation JSONB.

## Découpage en unités

| Unité | Portée | UI |
|---|---|---|
| `competition-rules-form` | Saisie des bonus dans le formulaire admin (phase-2 création compétition) : ajout agressif + seuil défensif configurable | Oui (mineure) |
| `post-match-bonus-calc` | Propagation ACL des règles + calcul des bonus dans `RankingLine::record_match` | Non |

Ordre de traitement : **`competition-rules-form` d'abord**, puis `post-match-bonus-calc`.

## Progression

| Unité | Mockup | Front | Back | DTOs | Use cases | Domaine | Intégration | Cartes |
|---|---|---|---|---|---|---|---|---|
| competition-rules-form | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ |
| post-match-bonus-calc | n/a | n/a | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | |