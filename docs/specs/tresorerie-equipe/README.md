# Trésorerie d'une équipe

**Épic :** aucune pour l'instant · **Maquette :**
`assets/rawpages/html/app-team-treasury.html`

## La fonction

L'onglet **Trésorerie** de la fiche équipe donne le relevé complet des
mouvements de caisse : d'où part l'équipe, ce qu'elle a encaissé, ce qu'elle a
dépensé, et pourquoi son solde est celui qu'il est. Un relevé de compte en
banque, pas un tableau de bord.

## Les pages

| Page | État |
|---|---|
| `onglet-tresorerie/` | phases 1 et 2 faites |

## Ce que la fonctionnalité suppose déjà acquis

`teams__treasury_ledger` existe et porte tout : `direction`, `amount_kpo`,
`reason`, **`balance_after_kpo`** et `occurred_at`. Rien n'est à recalculer.

Huit motifs sont définis dans le domaine (`teams/domain/treasury.rs`), sept ont
des lignes en base ; `CostlyMistake` attend son producteur (épic E13) et
s'affichera sans qu'on y revienne.
