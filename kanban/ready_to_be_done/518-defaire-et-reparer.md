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

- [ ] `UndoDrawCommand`, `RepairCommand`
- [ ] Les deux use cases, instrumentés
- [ ] `undo_draw` : suppression, `PairingDeleted` par rencontre, `defaire_appariement`
- [ ] `repair` : suppression de la touchée, écriture de la nouvelle,
      `marquer_appariee` avec la nouvelle exemptée
- [ ] R13 sur les deux
- [ ] Tests unitaires : défection avec exemptée disponible (elle reprend du
      service) · défection sans orphelin (l'adversaire devient exempté) ·
      R16 (arrivée tardive appariée avec l'exemptée) ·
      R13 (journée figée refusée)
- [ ] `make lint`, `make check-arch`, `make test`
