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

- [ ] `DrawCommand`, `ConfirmDrawCommand` (qui porte la proposition entière)
- [ ] Les deux use cases, instrumentés
- [ ] `interdites` construit depuis `ITeamInfoPort`
- [ ] `jamais_exemptees` construit depuis les campagnes de la saison (R9)
- [ ] L'émission après le commit, par `emettre()`
- [ ] Tests unitaires : R11 (journée déjà appariée) · R15 (moins de deux
      présents) · R18 (une désengagée est écartée de l'aperçu **et** refusée à la
      validation) · R22 (proposition falsifiée refusée)
- [ ] `make lint`, `make check-arch`, `make test`
