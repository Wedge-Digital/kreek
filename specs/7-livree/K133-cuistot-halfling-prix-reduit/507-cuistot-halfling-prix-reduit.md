# Le cuistot halfling coûte 300 kPo à tout le monde, halflings compris

**Priorité : haute — un prix faux, payé**
**Épic :** aucune
**Fichiers :** `src/app/references/domain/`,
`src/infrastructure/match_report/competition_data_adapter.rs`,
`src/app/references/io/web/inducement_selector_controller.rs`,
`assets/references.example/`

## Le défaut

Le maître cuisinier halfling coûte 300 kPo, sauf pour l'équipe halfling qui le
paie 100. L'application le facture 300 à tout le monde.

Le corpus porte pourtant la bonne donnée :

```json
{ "uid": "HALFLING_MASTER_CHEF", "cost": 300, "reducedCost": 100, … }
```

**`reducedCost` est désérialisé et lu nulle part.** Le seul code qui le
mentionne est un test de cohérence du corpus, qui vérifie qu'au moins un
inducement en porte un — jamais qu'il serve à quelque chose.

## Deux endroits, pas un

Le prix sort du référentiel à deux endroits, tous deux sur `cost` brut :

| Où | Ce qu'il décide |
|---|---|
| `competition_data_adapter.rs` · `build_inducement_spec` | le prix **débité**, via `TierRulesDto` |
| `inducement_selector_controller.rs` · `unit_cost: ind.cost` | le prix **affiché** |

**Corriger le seul affichage serait pire que le défaut** : le coach lirait 100
et se verrait prélever 300. C'est la forme exacte de divergence que le
`CLAUDE.md` documente à propos du format des kPo — un écran qui dit autre chose
que le domaine.

Rien n'est à faire circuler pour les corriger : `find_tier_rules_for_roster`
reçoit déjà le `roster_id` et ne le transmet pas à `build_inducement_spec`, et
le sélecteur reçoit le sien en paramètre d'URL.

## La règle est en dur, et c'est délibéré

Le corpus porte aussi `reducedCostFor: ["HALFLING_THIMBLE_CUP"]`, qui n'est
désérialisé nulle part. **Ce champ ne sera pas branché**, et le raisonnement
mérite d'être écrit parce qu'il a d'abord été fait à l'envers.

`HALFLING_THIMBLE_CUP` est une **ligue régionale** du corpus, portée par
`HALFLING` *et* `GNOME`. Brancher le champ tel quel donnerait donc le prix
réduit aux Gnomes, qui n'y ont pas droit. La première analyse de cette carte
concluait exactement cela, en lisant le corpus comme s'il énonçait la règle.

**Une donnée du corpus n'est pas une règle du jeu.** Le corpus dit ce qu'il
sait dire — des étiquettes de ligue — et la règle, elle, ne parle que de
l'équipe halfling. Elle ne changera pas ; elle est donc écrite dans le code, à
un seul endroit, sous un nom qui la désigne.

Ce qui reste lu dans le corpus est le **montant** (100), qui vit avec tous les
autres prix. Absent, on retombe sur `cost` : le code n'invente aucun chiffre.

## Le jeu de démo doit porter le couple

Une règle en dur nomme deux uid, et un e2e ne peut l'exercer qu'avec eux. Le
jeu de démonstration n'a ni cuistot ni équipe halfling : il reçoit donc
`HALFLING_MASTER_CHEF` et une équipe d'uid `HALFLING`.

**Ces deux entrées porteront un uid réel au milieu des `DEMO_*`.** C'est la
contrepartie assumée du choix ci-dessus. Elle ne dérange rien : `ROSTERS`, dans
`competition_lifecycle.py`, est une liste explicite de deux uid, et aucun test
ne compte les équipes du corpus.

## Ce que la carte ne fait pas

- Ni `BRIBES` ni `DODGY_LEAGUE_REP`, qui portent aussi un `reducedCost` — leur
  bénéficiaire est `BRIBERY_AND_CORRUPTION`, une vraie règle spéciale d'équipe,
  et c'est un autre sujet.
- Elle ne touche pas `assets/references/`, qui vient d'amont.
- **Elle ne corrige pas `restrictedTo`**, dont le filtre compare une règle
  spéciale (`APOTHECARY`, `MASTERS_OF_UNDEATH`, `FAVOURED_OF_NURGLE`,
  `LOW_COST_LINEMEN`) à un `roster_id`. Aucune de ces valeurs n'est un uid de
  roster : les quatre coups de pouce concernés sont donc exclus pour **toutes**
  les équipes, y compris celles qui y ont droit. Le défaut est invisible en
  test parce que le corpus de démo, lui, y met un roster (`DEMO_GRANIT`) — le
  code est juste sur le corpus qu'on exerce et faux sur celui qui tourne.
  Ça touche l'éligibilité, pas le tarif : sa propre carte.

## Tests

| Test | Ce qu'il prouve |
|---|---|
| `le_cuistot_est_reduit_pour_les_halflings` | 100 pour `HALFLING` |
| `le_cuistot_reste_plein_tarif_ailleurs` | 300 pour un autre roster |
| `sans_prix_reduit_le_tarif_ne_bouge_pas` | pas d'invention quand `reducedCost` manque |
| adapter : le `TierRulesDto` d'un halfling porte 100 | **le prix débité**, pas l'affiché |
| e2e : deux équipes achètent le cuistot | le **grand livre** montre −100 et −300 |

L'e2e lit la trésorerie, sur le modèle de `test_inducement_treasury.py`. Un
test qui lirait le prix à l'écran passerait alors même que le coach serait
prélevé de 300.

## Checklist

- [x] `cout_pour_roster` dans le domaine de `references`
- [x] Les deux appelants y passent
- [x] Jeu de démo : cuistot + équipe halfling
- [x] Les tests
- [x] `make lint && make test && make check-arch && make e2e`

`make test` : 1725 passés. `make e2e` : 371 passés, 7 sautés — un échec au
premier passage sur le flake de navigation connu de
`test_special_rule_selector`, qui nomme `DEMO_ZEPHYR` en dur et ne peut pas
dépendre du roster ajouté ; passé isolément, puis au second passage complet.

## Ce qui a été fait

`cout_pour_roster(inducement, roster_id)` vit dans
`references/domain/inducement_pricing.rs`, et les deux appelants y passent :
l'adapter du rapport de match (le prix **débité**) et le sélecteur (le prix
**affiché**).

### Ce que le corpus de démo a refusé

L'équipe halfling avait d'abord été écrite **sans ligue**, délibérément : pour
qu'aucun lecteur ne croie que le prix réduit vient d'une appartenance de ligue.

`example_dataset_exercises_optional_schema_fields` l'a refusée sur-le-champ, et
son commentaire disait pourquoi : *« une équipe ne peut pas être soumise sans
ligue (`LeagueNotSelected`) […] un roster de démo sans ligue serait injouable —
c'est arrivé, l'e2e l'a détecté »*. Un test écrit après un défaut passé en a
évité la répétition immédiate.

Elle a donc `LIGUE_DES_CIMES`, **la même que les Granitiers** — et l'intention
d'origine est mieux servie ainsi : le témoin du test partage la ligue du
halfling, donc un prix qui suivrait la ligue les rendrait identiques et ferait
échouer le test. La démonstration est portée par le test au lieu d'être confiée
à un lecteur attentif.

### Pourquoi l'e2e ne fait pas acheter le témoin

Premier jet : les deux équipes achètent le cuistot, et l'on compare les débits.
Il a fallu l'abandonner.

`inducement_budget_for` plafonne l'underdog à sa petite monnaie plus 50 kPo. Le
halfling, à 330 de valeur d'équipe contre 550, disposait de 270 — et son achat
à plein tarif était **refusé** plutôt que facturé trop cher. Le test mordait,
mais en disant « 422 » là où il fallait lire « 300 au lieu de 100 ».

L'e2e couvre donc **les deux points d'entrée du prix** — le tarif appliqué pour
le halfling, lu dans `match_report_proj`, et le tarif affiché pour les deux
rosters, lu au sélecteur par un `GET` que ni la petite monnaie ni la trésorerie
ne perturbent. C'est mieux que le premier jet : le prix affiché était l'autre
moitié du défaut, et rien ne le couvrait.

Le message d'échec de l'achat nomme désormais cette cause, puisqu'elle est ce
que le défaut produit.

### Vu mordre, deux fois

| Ce qu'on remet | Ce qui rougit |
|---|---|
| `unit_cost: ind.cost` dans l'adapter | l'achat du halfling, en 422 |
| la règle neutralisée dans `cout_pour_roster` | l'achat **et** le prix affiché |

Les tests unitaires de l'adapter éprouvent le **vrai jeu de démonstration** via
`load_for_tests()`, et non un mock : si le cuistot en disparaissait, ils le
diraient au lieu de continuer à prouver quelque chose sur un corpus imaginaire.
