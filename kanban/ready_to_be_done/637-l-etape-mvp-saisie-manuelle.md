# L'étape MVP, saisie manuelle

**Épic :** E21 — Le mode de saisie et l'après-match en séquence · **Ordre :** 7 · **Dépend de :** 631 ; en parallèle de 635-636
**Conception :** `docs/specs/saisie-mobile/E21-cadrage.md` (R7, R9, § 6, § 8.3, § 8.9)
**Maquettes :** `assets/rawpages/html/app-mobile-solo-mvp-home-manual.html`, `app-mobile-solo-mvp-away.html`

## Objectif

Le MVP devient **la première étape de l'après-match** : domicile, puis
visiteur, un joueur choisi directement. Il s'inscrit au journal comme une action
`Mvp` au **tour 16**, **un par camp**. Le bouton ⭐ quitte le panneau d'action
(`templates/action-panel-widget.html:147-150`). L'indicateur passe à cinq
étapes : Avant-match · Match · MVP · Gains et fans · Validation.

## Conception

**Domaine** — dans `MatchReportPreMatch` :

```rust
pub fn record_mvp(&self, side: TeamSide, player: ActionPlayer, player_display_name: String,
    player_position: String, action_id: ActionId, recorded_by: CoachId)
    -> Result<(Self, MatchReportDomainEvent), DomainError>;
```

Tour 16 fixé ; refus `MvpAlreadyRecorded` si le camp a déjà un `Mvp`, quel que
soit son tour. `record_action` refuse `Mvp` (`MvpOutsideMvpStep`) — décision
du 2026-11-01 : la règle n'a pas de porte de service.
`record_two_mvp_same_team` (`match_report_pre_match.rs:1132`) est réécrit en
conséquence.

**Le rejeu n'est pas touché** : il construit l'action sans passer par
`record_action` (`domain/match_report_state.rs:189-215`, `359-385`). Un rapport
ancien à deux MVP, ou à un MVP au tour 7, se relit à l'identique — un test le
fixe.

**Use case** `record_mvp_use_case`, sur le modèle de `record_action_use_case`
(résolution du nom et du poste, `ReadyToPublish` → `into_pre_match` en
correction, `Conflict`).

**Routes** `GET/POST /{id}/mvp/{side}`, dans `routes_de_saisie()`. La page :
bannière du camp, « MVP 1 / 2 » ou « 2 / 2 », le sélecteur de joueurs du BC
`players` (déjà utilisé en `step3`) et les joueurs temporaires du camp ; au
visiteur, le MVP du domicile replié en tête. Un camp qui a déjà son ou ses MVP
— correction d'un ancien rapport — les montre, avec ✕ ; on supprime avant d'en
saisir un autre.

**Enchaînements** : « Fin du match → MVP » (636) → `/mvp/home` → `/mvp/away` →
`step5`, dont le retour mène à `/mvp/away` (`step5_controller.rs:250`).
La Validation est le récapitulatif non publié, indicateur à 5, sans nouvelle
route (décision du 2026-11-01).

## Données de production

Aucune. Les anciens MVP — à tout tour, zéro ou plusieurs par camp — restent
valides : la règle ne vaut qu'à la saisie, sinon des comptes rendus publiés
deviendraient illisibles. SPP et événement publié inchangés. Les comptes rendus
en cours ne sont pas bloqués : leur MVP déjà saisi s'affiche à l'étape.

## Checklist

- [ ] `record_mvp`, ses erreurs ; `record_action` refuse `Mvp`
- [ ] `record_mvp_use_case`, `/mvp/{side}`, gabarit et feuille au bundle
- [ ] Bouton ⭐ retiré ; `"MVP"` retiré du parseur d'action
- [ ] Enchaînements et indicateur ; récapitulatif non publié à l'étape 5
- [ ] Tests unitaires : tour 16 ; second MVP refusé ; rejeu d'un rapport ancien à deux MVP et à un MVP hors tour 16
- [ ] E2E : saisir le MVP domicile puis visiteur ; `record_mvp_api` dans `match_report_helpers.py` ; migration des tests qui saisissent `MVP` comme action — `test_match_report_recap.py`, `test_match_report_recap_spp.py`, `test_match_report_correction.py`, `test_manual_phase_override.py`, `test_player_spp_spending.py` (deux MVP dans un camp), `test_player_detail.py`, `test_match_report_step3_4_actions.py` (S7, S8)
- [ ] `tests/impact-map.toml` et `tests/e2e/visual/urls.py` mis à jour
- [ ] `make lint`, `make check-arch`, `make test`, e2e impactés
