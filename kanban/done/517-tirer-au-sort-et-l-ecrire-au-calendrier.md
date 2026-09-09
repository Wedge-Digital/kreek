# Tirer au sort, et l'écrire au calendrier

**Priorité : haute — c'est la promesse de la fonction**
**Épic :** E16 — Sondage de présence
**Dépend de :** 507 (le tirage), 509 (l'écriture atomique), 510, 513
**Fichiers :** `src/app/competitions/use_cases/presences/draw_pairings_use_case.rs`,
`confirm_draw_use_case.rs`

## `draw_pairings_use_case` — un POST qui n'écrit rien

1. charge la campagne et la journée
2. refuse si la journée porte déjà des appariements — `PairingsAlreadyExist` (R11)
3. retient les présents (R5), écarte les désengagés via `filter_enrolled_team_ids` (R18)
4. refuse en dessous de deux — `NotEnoughPresent { presents }` (R15)
5. construit `DrawInput` et appelle `tirer`
6. rend `DrawProposal` — **rien n'est écrit**

**C'est le use case qui remplit `interdites`**, parce que la relation équipe ->
coach vit dans un autre BC et arrive par `ITeamInfoPort`. Le domaine reçoit des
paires interdites ; il n'a pas à savoir qu'un coach existe. R10 est métier, sa
**matière** est inter-BC.

**L'aperçu filtre aussi les désengagées** (R18) : sans quoi il proposerait une
rencontre que la validation refuserait, et l'organisateur verrait son tirage
changer sans avoir rien fait.

**Instrumenté comme les autres** bien qu'il n'écrive rien : c'est une action de
l'organisateur, et le journal doit la porter. Pas de `arch:no-instrument` ici —
le marqueur est réservé aux services d'hydratation.

## `confirm_draw_use_case`

1. recharge campagne et journée, revalide R11 et R18
2. `survey.valider_proposition(...)` — l'agrégat **revérifie** (R22)
3. construit les `Pairing` et leurs projections (`build_new_pairing_projection`)
4. `match_day_repo.save_pairings(...)` — transaction unique, verrou sur la journée
5. **après le commit**, émet un `PairingCreated` par rencontre via `emettre()`
6. `survey.marquer_appariee(exemptee)`, persiste

**L'émission vient après le commit, jamais dedans.** Un listener qui réagit à un
`PairingCreated` dont la transaction est ensuite annulée aurait travaillé sur un
fait qui n'a pas eu lieu, et rien ne le lui dirait. L'ordre inverse paraît plus
naturel — « tout dans la même unité » — et c'est le piège.

**Toute émission passe par `emettre()`.** Un `.send(` direct reprendrait
l'identifiant de l'enveloppe reçue au lieu de celui que `to_enveloppe()`
engendre, et produirait une trace qui a l'air correcte sans rien corréler.

**Aucun événement à créer** : `PairingCreated` existe et le publisher le
convertit déjà. Conséquence à noter — le tirage entré par la porte du sondage
produit en aval exactement les mêmes effets que celui du Calendrier.

## Checklist

- [x] `DrawCommand`, `ConfirmDrawCommand` — **sans l'étiquette**, cf. ci-dessous
- [x] Les deux use cases, instrumentés
- [x] `interdites` construit depuis `ITeamInfoPort`
- [x] ~~`jamais_exemptees`~~ → **`matchs_joues`**, la carte 541 ayant remplacé le
      champ : le critère se lit sur les appariements de la saison
- [x] L'émission après le commit, par `emettre()`
- [x] Tests unitaires : les quatre de la liste, plus R10, l'historique recalculé,
      l'absence d'émission sur écriture en échec — 13 tests
- [x] `make lint`, `make check-arch`, `make test` — 1840/1840
- [x] `make e2e` — **368/368**, suite complète sur base au gabarit

## Ce que la réalisation a tranché

**La commande ne porte pas d'`Historique`.** `ProposedPairing` en a un — « 1re
rencontre », « 2e rencontre · J1 » — mais il est **dérivé** de la saison. Le
laisser entrer par la commande permettrait à une proposition falsifiée d'afficher
« inédite » sur une revanche : le libellé viendrait du navigateur au lieu des
journées. La commande ne porte que les couples et l'exemptée ; le use case
recalcule l'étiquette avant de soumettre le tout à l'agrégat.

**Deux modules partagés, par déplacement.** `entree_du_tirage.rs` reçoit
`build_interdites`, `meme_coach`, `build_historique` et `build_matchs_joues` —
avec les trois tests de la dernière. Les recopier aurait fait **deux tirages qui
divergeraient**, ce que la 541 venait précisément de réparer d'un seul côté.

`appariement_ecrit.rs` reçoit l'émission, **réécrite pour lire la projection**
plutôt que `team_display` : `NewPairingProjection` porte exactement les quatorze
champs d'affichage de l'événement. Deux `expect()` disparaissent, et l'événement
ne peut plus diverger de ce que la base contient — les deux sortent de la même
valeur. `generate_pairings` en profite et perd cinq imports.

**Deux transactions, assumées.** `save_pairings` écrit la journée, `save` écrit la
campagne : deux agrégats, deux dépôts. Une panne entre les deux laisse la journée
appariée et la campagne se croyant vierge. **Ce n'est pas la règle « projection
dans la même transaction »** — il ne s'agit pas d'un événement et de sa
projection. Et la conséquence est bénigne : `desaccord` lit l'appariement sur la
journée (R24), donc l'écran reste juste ; seule l'exemptée apparaîtrait à tort
comme orpheline, et re-valider ou réparer le corrige. Bâtir un dépôt qui écrit
les deux tables coûterait un objet connaissant deux agrégats, pour un défaut qui
se voit et se répare.

**L'axe 12 a fait échouer `check-arch` sur un commentaire.** La doc citait le nom
de la méthode du bus ; l'axe la cherche **sans retirer les commentaires**,
contrairement à l'axe 9 — dont le `CLAUDE.md` explique justement pourquoi il les
retire : « un verrou qui hurle sur sa propre documentation se fait désactiver dans
la semaine ». Prose reformulée et écart noté sur place, plutôt qu'un `arch:ok` sur
du texte.
