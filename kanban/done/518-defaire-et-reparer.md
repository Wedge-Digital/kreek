# Défaire, et réparer

**Priorité : moyenne — le sixième état de la maquette**
**Épic :** E16 — Sondage de présence
**Dépend de :** 517
**Fichiers :** `src/app/competitions/use_cases/presences/undo_draw_use_case.rs`,
`repair_pairing_use_case.rs`

## `undo_draw_use_case`

Supprime les appariements de la journée, émet un `PairingDeleted` par rencontre,
rend la campagne à l'état clos par `defaire_appariement`. **Refuse sur journée
figée** (R13) : on ne défait pas un tirage dont un match est déjà rapporté.

## `repair_pairing_use_case` — R12 et R16

L'équipe qui se décommande sort ; sa rencontre tombe ; son adversaire rejoint le
vivier des orphelins et le système propose de le réapparier. **Les autres matchs
ne bougent pas** : leurs coachs ont déjà noté leur adversaire, et refaire le
tirage entier pour une défection ferait trois mécontents pour en soulager un.

**L'exemptée est le premier remplaçant considéré.** C'est ce qui distingue cette
règle de l'alternative « l'adversaire devient exempt » : sur le cas de la
maquette, le retirage partiel garde quatre matchs là où l'autre règle en aurait
perdu deux — l'orphelin *et* l'exemptée restant sur le banc. Si aucun orphelin
n'est disponible, l'adversaire devient exempté — et R9 ne s'applique pas ici :
c'est une exemption subie, pas tirée.

**R16 est le même mécanisme en sens inverse** : l'arrivant tardif rejoint les
orphelins et s'apparie avec l'exemptée s'il y en a une. Écrire R12 sur la seule
défection aurait produit un domaine sachant retirer une équipe et pas en ajouter
une.

## Ce que la réparation partage avec la validation

Mêmes gardes que `confirm_draw` : la proposition de réparation est revérifiée
comme la proposition initiale (R22), et l'écriture passe par la même méthode
transactionnelle. `marquer_appariee` est rappelée avec la **nouvelle** exemptée —
celle qui a finalement eu lieu (R9), pas celle qui était proposée.

## Checklist

- [x] `UndoDrawCommand`, `RepairCommand`, **`ProposeRepairCommand`**
- [x] **Trois** use cases, instrumentés — le calcul de la proposition est venu ici
- [x] `undo_draw` : suppression, `PairingDeleted` par rencontre, `defaire_appariement`
- [x] `repair` : suppression des touchées, écriture des nouvelles,
      `marquer_appariee` avec la nouvelle exemptée
- [x] R13 sur les trois
- [x] Tests unitaires : les quatre de la liste, plus R18 sur le vivier, R22 sur une
      réparation falsifiée, les autres matchs intacts — 13 tests
- [x] `make lint`, `make check-arch`, `make test` — 1855/1855
- [x] `make e2e` — **368/368**

## Ce que la réalisation a simplifié

**La réparation *est* un tirage sur le vivier.** Cette carte décrivait un
algorithme propre à la réparation — l'exemptée d'abord considérée, sinon
l'adversaire devient exempté. C'est exactement ce que `tirer` fait déjà : R8.1
apparie le plus possible, R8.2 évite les revanches, R10 refuse les paires
interdites, et ce qui reste est exempté.

« L'exemptée est le premier remplaçant considéré » n'est donc pas une règle à
écrire : **elle est dans le vivier**, et R8.1 la fait servir parce qu'un
appariement de plus vaut mieux qu'une exemption. « Sinon l'adversaire devient
exempté » est le cas où le vivier ne contient que lui.

R16 tombe avec : `desaccord` range l'arrivant dans les orphelins, et le vivier
les traite tous pareil. Rien à écrire pour le sens inverse.

Le bénéfice dépasse l'économie : un second algorithme aurait divergé du premier
au premier changement de règle. Celui-ci ne le peut pas — c'est le même.

**`undo_draw` enrobe `clear_round`**, qui existait déjà et vide une journée en
refusant ce qui est rapporté. R13 y est vérifiée **avant** la suppression : s'en
remettre au compte rendu de `clear_round` laisserait un tirage à moitié défait —
trois rencontres supprimées, une gardée, et une campagne qui ne sait plus où elle
en est.

**`valider_proposition` reçoit toute la journée après réparation**, pas le
fragment réécrit. Sa vérification de parité (R22) regarde les présents que rien
n'apparie : lui donner la seule rencontre réparée la ferait crier sur les autres
matchs, légitimes et intouchés.

**`save_pairing` et non `save_pairings`** : la plurielle refuse une journée qui
porte déjà des appariements — R11, sous son verrou — et c'est le cas ici.

## Un défaut trouvé dans du code livré

Deux tests ont échoué au premier jet, et c'était `tirer` qui avait tort :

```rust
if input.equipes.len() < 2 {
    return DrawProposal::default();   // exemptee: None
}
```

**Le raccourci mentait.** La recherche range dans `exemptees` toute équipe
qu'elle n'apparie pas, la dernière comprise ; le court-circuit rendait « aucune
exemptée » pour une équipe seule. Sans conséquence tant que les deux appelants
garantissaient au moins deux équipes — `filter_enrolled_team_ids` pour le
Calendrier, `peut_tirer` pour le sondage. La réparation d'une journée où il ne
reste qu'un orphelin l'a fait sortir : elle annonçait que personne n'était sur le
banc alors que quelqu'un y était.

Corrigé, avec deux tests dans `tirage.rs` — une équipe seule est exemptée, un
effectif vide n'exempte personne.
