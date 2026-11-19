# « Générer une journée complète » passe au nouveau tirage

**Épic :** E18 — Appariement des présents · **Ordre :** 4 · **Dépend de :** 612, 613, 614
**Conception :** `docs/specs/appariement-des-presents/calendrier/05-use-cases.md` (§ chargement commun, § 4)

## Le défaut corrigé

`generate_round_pairings` (`domain/match_day.rs`) n'a aucun aléa : il prend les
paires dans l'ordre des indices, gloutonnement, et **préfère laisser une équipe
sans match plutôt que de programmer une revanche**. Mesuré sur la branche
`sondage-presence` : 54 à 58 % des tirages laissent des équipes sur le banc en
milieu de saison.

```
4 équipes, déjà joué A-B, A-C, B-C  →  un seul match, A contre D
                                        B et C repartent sans jouer
```

Cette carte se livre seule : elle répare « Générer les rencontres » et
« Générer toutes les rencontres » avant que l'écran des présents existe.

## Conception

**`use_cases/admin/draw/draw_input.rs`** — copié de `entree_du_tirage.rs`
(branche), renommé : `build_history`, `build_forbidden_pairs`,
`build_matches_played`. Plus `DrawContext` et `load_draw_context`, le
chargement commun (journée, inscrites via `ITeamInfoPort`, poules avec repli
« Toutes les équipes », journées de la saison), et la construction de
`DrawInput` et `DrawRules` pour une liste d'équipes.

**`use_cases/admin/pairing_created.rs`** — copié de `appariement_ecrit.rs`
(branche) : l'émission de `PairingCreated`. Remplace `emit_pairing_created`.

**`generate_pairings::execute`** — signature et `GenerateOutcome` inchangées :

1. `load_draw_context`, `match_day.can_receive_draw()` ;
2. par poule, `DrawInput` sur **toutes** les inscrites ; équipes et poules
   ignorées toujours signalées ;
3. `draw::draw_pools`, puis `match_day.accept_draw` ;
4. **un seul `save_pairings`** pour la journée (R14) ;
5. `PairingCreated` après le commit.

**`schedule-round-detail.html`** — « Générer les rencontres » devient
« Générer une journée complète », et n'apparaît que si `detail.fixtures` est
vide (R9).

## Ce qui est retiré — consommateurs vérifiés (règle 4)

| Élément | Consommateurs | Remplacé par |
|---|---|---|
| `generate_round_pairings`, `normalize_pair` et leurs tests | `generate_pairings.rs` seul | `draw::draw` et ses tests (612) |
| `build_played_set`, `generate_and_save_group_pairings` | locaux | `draw_input.rs`, `save_pairings` |
| `emit_pairing_created` | local | `pairing_created.rs` |

Revérifier les consommateurs au moment de supprimer. Le commentaire de
`tests/e2e/test_invariant_journee.py` qui cite `generate_round_pairings` est
mis à jour.

## Checklist

- [ ] `draw_input.rs` et `pairing_created.rs` copiés, renommés
- [ ] `generate_pairings` réécrit en cinq étapes, fonctions sous vingt lignes
- [ ] Les trois éléments retirés, consommateurs revérifiés
- [ ] Libellé et condition d'affichage du bouton
- [ ] Tests du use case : toutes les inscrites appariées hors l'exemptée sur
      l'historique de quatre équipes ; deux équipes d'un même coach jamais
      opposées ; journée pourvue refusée ; « Générer toutes les rencontres »
      passe par le même tirage (R12)
- [ ] e2e n°9 : « Générer une journée complète » ne laisse aucune équipe sans
      match hors l'exemptée ; `tests/impact-map.toml`
- [ ] Commentaire de `test_invariant_journee.py`
- [ ] `make lint`, `make check-arch`, `make test`, e2e impactés
