# Des achats validés ne se modifient plus

**Épic :** E20 — Coups de pouce : le panier et l'annulation · **Ordre :** 2 · **Dépend de :** 626
**Conception :** `docs/specs/saisie-mobile/E20-cadrage.md` (R1, R2, R7, R9)

## Objectif

Le budget de l'outsider dépend des dépenses du favori (`petty_cash`,
`match_report_pre_match.rs:159`). Si le favori revient modifier ses achats, ce
budget change sous l'outsider sans que rien ne le revalide. Après la carte, une
équipe qui a validé — achats ou « Passer » — ne peut plus rien enregistrer ;
l'erreur se corrige par l'annulation (626).

**Et le favori achète d'abord** (R9, décision du 2026-11-01) : tant qu'il n'a pas
validé, l'outsider ne peut rien enregistrer — sans quoi son budget se
calculerait sans les dépenses du favori.

Le gel referme au passage deux chemins fautifs : racheter en `ReadyToPublish`
sans que la dépense figée suive (`match_report_state.rs:302-316`), et racheter
après des actions qui désignent des joueurs temporaires remplacés.

## Conception

**« A validé »** se lit dans l'agrégat : `inducements_for(team).is_some()`
(`match_report_pre_match.rs:229`). « Passer » enregistre un
`InducementsRecorded` vide (`inducements_controller.rs:208-217`,
`match_report_pre_match.rs:625-634`) : il vaut validation. Exposer
`has_recorded_inducements(&TeamId) -> bool`.

**Domaine** — `record_inducements` refuse
`DomainError::InducementsAlreadyRecorded` en tête, avant toute validation
d'achat. Vaut aussi depuis un `ReadyToPublish`, que le use case convertit par
`into_pre_match()`.

**Use case** — `RecordInducementsError::AlreadyRecorded`, distinct de
`Domain(..)`, pour que le contrôleur rende **409** et non 422 : ce n'est pas une
saisie invalide, c'est un état. Message : « Les coups de pouce de cette équipe
sont validés. Pour les changer, annulez ceux des deux équipes. »

**R9** — `record_inducements` refuse aussi l'outsider tant que le favori n'a
pas validé : `DomainError::TopdogHasNotChosen`, relayé en
`RecordInducementsError::TopdogFirst`, **409**, message « Le favori choisit ses
coups de pouce en premier. » Le favori se lit par `topdog_team_id()`, sa
validation par `has_recorded_inducements`.

**GET** — l'outsider dont le favori n'a pas validé : `303` vers la page du
favori (R9). Si l'équipe demandée a validé et l'autre non : `303` vers la page de
l'autre (R7, décision 2 du 2026-11-01). Le cas « les deux ont validé » est
celui de 628 ; d'ici là, même redirection vers le favori.

## Données de production

Aucune : l'indicateur se reconstruit au rejeu. **R9 sur l'existant** : un compte
rendu en cours dont l'outsider n'a pas encore acheté et le favori non plus
renvoie l'outsider vers le favori ; un outsider qui avait déjà acheté **avant**
le favori garde ses achats — R9 ne s'applique qu'aux nouveaux envois. Les comptes rendus en cours dont
une équipe a validé sont figés pour elle **dès la livraison**, en `PreMatch`
comme en `ReadyToPublish` et après dépublication. Un outsider qui aurait acheté
avant le favori garde ses achats.

## Checklist

- [x] `has_recorded_inducements`, le refus du domaine
- [x] `AlreadyRecorded`, 409 et son message ; `refus_d_achat` le nomme
- [x] La redirection du GET
- [x] Tests unitaires : racheter après un achat, après « Passer », depuis un
      `ReadyToPublish` ; l'autre équipe achète toujours ; après une annulation
      (626), on rachète
- [x] R9 : le refus de l'outsider avant le favori, 409, et la redirection du GET ;
      tests unitaires : l'outsider refusé avant le favori, admis après un achat
      comme après « Passer »
- [x] `test_inducements_restreints`, `test_inducements_tarif_reduit` et
      `test_cuistot_halfling` font passer le favori d'abord quand l'équipe visée
      est l'outsider
- [x] `test_cart_footer_visible` rendu indépendant du passage du favori dans le
      même module (cf. fiche § 6)
- [x] `make lint`, `make check-arch`, `make test`, e2e impactés
