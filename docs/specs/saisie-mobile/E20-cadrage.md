# E20 — Coups de pouce : le panier et l'annulation — cadrage

Fiche de cadrage de l'épic `kanban/epics/to_be_refined/E20-coups-de-pouce-panier-et-annulation.md`,
**brouillon du 2026-11-01, à valider**. Le découpage en cinq cartes et les
décisions du 2026-11-01 sont repris de l'épic ; la fiche part de ce que le code
fait, pose les règles, les événements, les routes, les données de production et
les tests. Les six points ouverts ont été tranchés le 2026-11-01 (§ 2).

## 1. Ce que le code fait aujourd'hui

### L'agrégat d'avant-match

- **« A validé » se lit déjà dans l'agrégat.** `home_inducements` /
  `away_inducements` sont des `Option<Vec<InducementPurchase>>`
  (`domain/match_report_pre_match.rs:47-48`) : `None` tant que l'équipe n'a rien
  enregistré, `Some(..)` dès un `InducementsRecorded` — le rejeu pose `Some` sans
  condition (`domain/match_report_state.rs:132-146`). La méthode privée
  `inducements_for` (`match_report_pre_match.rs:229`) rend cette `Option` ;
  `purchases_for` (`:372`) l'aplatit en tranche vide et perd la distinction.
- **« Passer » enregistre bien un `InducementsRecorded` vide.**
  `resolve_purchases_from_form` rend deux listes vides quand `intent == "pass"`
  (`io/web/inducements_controller.rs:208-217`) ; `record_inducements` les valide
  puis `build_inducement_events` émet **toujours** un `InducementsRecorded`, même
  vide (`match_report_pre_match.rs:625-634`). L'e2e
  `test_pass_discards_pending_selection` lit `[]` dans la projection
  (`tests/e2e/test_match_report_inducements.py:224-259`).
- **Le favori et la petite monnaie** : `topdog_team_id` (`:121`) — la règle vit
  dans `underdog::top_dog` ; `topdog_spending` (`:140`) somme les achats du
  favori, `unwrap_or(&[])` sur un `None` ; `petty_cash` (`:159`) = écart de
  valeur d'équipe + dépenses du favori ; `inducement_budget_for` (`:170`) : le
  favori a sa trésorerie, l'outsider `petty_cash + min(trésorerie, 50)`.
  **Annuler les achats du favori ramène donc mécaniquement la petite monnaie à
  l'écart de valeur** : rien à recalculer ailleurs.
- **Les contrôles d'achat** — `validate_purchases` (`:488-499`) : quantité
  maximale, deux vedettes, vedette déjà engagée par l'adversaire, budget, trois
  mercenaires. Aucun ne regarde si l'équipe a déjà validé.
- **Un nouvel enregistrement remplace l'ancien** : le rejeu retire les vedettes
  de l'équipe (`star_engagements.retain`, `match_report_state.rs:137`) avant que
  les `StarPlayerEngaged` qui suivent ne les reposent (`:148-161`).
- **Les joueurs temporaires** (vedettes, mercenaires, journaliers) naissent dans
  `init_temp_players_use_case`, appelé par `record_inducements_use_case` juste
  après son append (`record_inducements_use_case.rs:407-434`). S'il en existait
  déjà, `reset_if_needed` appende d'abord un `TempPlayersReset`
  (`init_temp_players_use_case.rs:108-136`), qui **nomme les journaliers
  retirés** (`match_report_pre_match.rs:259-280`). Les deux événements sont
  émis par `emettre()` (`:85`, `:134`) ; le publisher en fait
  `JourneymenFielded` / `JourneymenWithdrawn` pour `players`
  (`io/app_events/app_event_publisher.rs:133-138`, `:266-291`).
- **`InducementsRecorded` et `StarPlayerEngaged` ne sont pas émis** sur le bus :
  `persist_and_init` les appende sans `emettre()`. Aucun BC ne les écoute ;
  l'argent ne sort qu'à la publication, depuis l'état `ReadyToPublish`
  (`InducementSpending`, `match_report_ready_to_publish.rs:128-133`).
- **Les actions de match** vivent dans `home_actions` / `away_actions`
  (`match_report_pre_match.rs:52-53`), alimentées par `ActionRecorded` et
  retirées par `ActionDeleted` (`match_report_state.rs:189-232`).

### Le cycle de vie, et où l'on peut encore acheter

- Les écritures d'avant-match acceptent **`PreMatch` et `ReadyToPublish`** : les
  deux use cases et le contrôleur convertissent un `ReadyToPublish` par
  `into_pre_match()` (`record_inducements_use_case.rs:196-200`,
  `init_temp_players_use_case.rs:101-105`, `inducements_controller.rs:163-167`).
  Un rapport **dépublié** revient en `ReadyToPublish` (`match_report_state.rs:446-453`) :
  ses coups de pouce redeviennent modifiables.
- **Deux défauts que le gel referme, sans qu'on les ait cherchés :**
  1. En `ReadyToPublish`, un nouvel `InducementsRecorded` change les achats
     (`match_report_state.rs:302-316`) **mais pas** `home_inducement_spending` /
     `away_inducement_spending`, figés à l'entrée dans cet état
     (`match_report_ready_to_publish.rs:128-133`). Racheter après l'étape 5
     débiterait l'ancien montant à la publication.
  2. Racheter **après des actions** relance `init_temp_players` : les joueurs
     temporaires reçoivent de nouveaux identifiants
     (`init_temp_players_use_case.rs:145`, `:166`), et les actions qui
     désignaient les anciens (`ActionPlayer::Temp`, `value_objects.rs:286`)
     pointent dans le vide.
- **Un match peut aller jusqu'à la publication sans aucune action** :
  `record_post_match_use_case` n'exige ni action ni MVP (`:61-92`). L'épic
  justifie la perte de capacité par « un match publié ayant toujours des
  actions » — **ce n'est pas vrai dans le code**. La règle R3 ci-dessous
  l'obtient autrement : par l'état.

### L'écran

- `inducements.html` : le pied de page est un `<form method="post">` (`:97`) ;
  il ne liste **que les mercenaires** (`:103-117`), le reste n'est qu'un compte
  (`:120-122`). Le budget est fixé au rendu (`:10`).
- Le sélecteur (`references/io/web/templates/widgets/inducement-selector.html`)
  tient les quantités dans son `x-data` et publie `inducementSelectionChanged`
  `{ instanceId, items: [{uid, name, qty, unit_cost}], total_cost }` (`:127-135`).
  Il **n'écoute rien** qui lui demanderait d'ajuster une quantité.
- Le widget des mercenaires écoute `removeMercenaire` par un
  `document.body.addEventListener` posé dans `init()`
  (`match_report/widgets/mercenary-selector-widget.html:84`), **sans retrait** :
  l'écouteur survit au swap. La carte 629 le remplace par un `@….window`.
- La page s'ouvre **toujours vide** (`items: []`, `:8`) : revenir sur une équipe
  qui a validé ne montre pas ses achats, on rachète de zéro.
- La garde `garde_saisie` (E19, carte 622) couvre déjà
  `GET/POST /inducements/{team_id}` (`router.rs:93-96`).

## 2. Les règles

| # | Règle |
|---|---|
| R1 | Une équipe **a validé** ses coups de pouce quand un `InducementsRecorded` la concerne depuis le dernier `InducementsCancelled` — achats ou « Passer ». Lu dans l'agrégat : `inducements_for(team).is_some()`. |
| R2 | Une équipe qui a validé **ne peut plus enregistrer d'achats** : le domaine refuse, la route répond **409** avec un message qui renvoie à l'annulation. Vaut en `PreMatch` comme en `ReadyToPublish`. |
| R3 | **Annuler les coups de pouce** efface les achats et les vedettes des **deux** équipes, et retire leurs joueurs temporaires. Permis seulement si le rapport est en **`PreMatch`** et **qu'aucune action n'y est enregistrée** — actions supprimées non comprises. |
| R4 | Au moins une équipe doit avoir validé pour annuler ; sinon refus (rien à annuler). |
| R5 | Après annulation, on recommence **par le favori** ; le budget de l'outsider se recalcule de lui-même (`petty_cash`, § 1). |
| R6 | Quand les **deux** équipes ont validé, `GET /inducements/{team_id}` rend le **récapitulatif** des deux camps — quel que soit `team_id` — avec le coup d'envoi et, si R3 le permet, l'annulation. |
| R7 | Quand **une seule** équipe a validé, `GET /inducements/{team_id}` de cette équipe **redirige** vers la page de l'autre. |
| R9 | **Le favori achète d'abord** : tant qu'il n'a pas validé (achats ou « Passer »), l'outsider ne peut rien enregistrer — le domaine refuse, la route répond **409** —, et `GET /inducements/{outsider}` **redirige** vers la page du favori. Sans cela, le budget de l'outsider (`petty_cash`) se calculerait sans les dépenses du favori. |
| R8 | Le panier liste **tous** les articles ; chacun s'ajuste ou se retire depuis le panier. Le formulaire envoyé ne change pas. |

**Pourquoi R3 s'arrête à `PreMatch`.** En `ReadyToPublish`, la dépense de
trésorerie est figée (§ 1, défaut 1) : annuler là exigerait de la recalculer.
Et un rapport **dépublié** est en `ReadyToPublish` : la perte de capacité
décidée le 2026-11-01 découle de cette borne, **pas** de la présence d'actions,
qui n'est pas garantie.

**Décisions du 2026-11-01** — les six points ouverts du brouillon :

1. **La borne de R3** : « `PreMatch` et aucune action **vivante** » — une action
   supprimée ne désigne plus personne.
2. **R7, l'équipe qui a validé seule** : la page **redirige** vers l'autre
   équipe ; la vue figée d'attente est l'affaire de E22.
3. **L'ordre favori puis outsider est imposé** (R9) — sinon la règle de calcul
   du budget de l'outsider casse. Trois e2e achetaient pour une seule équipe sans
   regarder qui est favori (`test_inducements_restreints`,
   `test_inducements_tarif_reduit`, `test_cuistot_halfling`) : ils font passer
   le favori d'abord (carte 627). À valeurs d'équipe égales, le favori reste le
   domicile (`>` strict) — la carte `274-inducements-egalite-de-tv` garde cette
   question ouverte, elle n'est pas tranchée ici.
4. **La seconde validation** mène au **récapitulatif** (628) ;
   `test_underdog_pass_redirects_to_step3` change d'attente.
5. **« par toi », « par Morgane »** : l'auteur de chaque validation est **gardé
   dans l'agrégat**, un champ par camp sur `PreMatch` et `ReadyToPublish`,
   reconstruit au rejeu depuis `recorded_by` — aucune donnée touchée.
6. **La catégorie au récapitulatif** : « Coup de pouce », « Star » (via
   `star_engagements`) ou « Mercenaire » / « Mercenaire Niv. 1 » (préfixe
   `MERCO:`) ; le port n'est pas étendu.

**Conséquence pour E22** : son « J'ai fini mes achats » se réduit à l'écran
d'attente et à son direct (E23) — la validation du favori, c'est déjà son
`InducementsRecorded`, et R9 la rend bloquante dès E20.

**Hors E20** : « Morgane est prévenue » (maquette d'annulation) relève des
notifications, épic E23 ; la phrase est retirée de l'écran. Qui peut annuler :
la garde E19 suffit — admin ou coach de l'une des deux équipes ; la
restriction par camp est l'affaire de E22.

## 3. Les événements

**Un ajouté, aucun modifié.**

```rust
/// Les coups de pouce des deux équipes sont effacés : on recommence par le favori.
InducementsCancelled { cancelled_by: CoachId },
```

- **Rejeu** (`PreMatch` seulement — R3) : `home_inducements = None`,
  `away_inducements = None`, `star_engagements` vidé. Dans tout autre état :
  `InvalidEventSequence`, comme le reste de `rehydrate`.
- **Les joueurs temporaires ne passent pas par lui** : le use case appende,
  dans le même `append_many`, un `TempPlayersReset` **existant** par camp qui en
  a. On réutilise ainsi tel quel le chemin qui retire les journaliers de
  `players` (`JourneymenWithdrawn`) et la projection (`'[]'::jsonb`), éprouvés
  par la carte 455. Le nom dit ce qui s'est passé dans le domaine, pas d'où vient
  la demande.
- **Projection** (`match_report_repository.rs`, `match` exhaustif sans joker) :
  un bras qui remet `home_inducements` et `away_inducements` à `NULL` — la
  colonne est nullable (`migrations/20260626000001_…sql:4`).
- **Émission** : `emettre()` pour l'`InducementsCancelled` et pour chaque
  `TempPlayersReset`. Le publisher n'a rien à faire du premier — son
  `aiguiller` l'ignore (`app_event_publisher.rs:150`) — mais l'axe 18 exige
  qu'il soit émis.
- `type_name()` : un bras de plus.

## 4. Les routes

| Route | Aujourd'hui | Après E20 |
|---|---|---|
| `GET /inducements/{team}` | page de choix, vide, en `PreMatch` comme en `ReadyToPublish` | choix si l'équipe n'a pas validé ; redirection vers l'autre (R7) ; récapitulatif quand les deux ont validé (R6) |
| `POST /inducements/{team}` | remplace les achats, à tout moment avant publication | **409** si l'équipe a validé (R2) ; la seconde validation mène au récapitulatif décision 4 |
| `POST /{id}/cancel-inducements` | — | **nouvelle**, dans `routes_de_saisie()` (garde E19) ; `HX-Redirect` vers `/inducements/{favori}` ; **409** hors R3 |
| `POST /step2` | rejoue le facteur fans puis renvoie au favori | inchangé ; la page d'arrivée applique R6/R7 |
| `/step3`, `/step4`, `/step5`, `/recap` | — | inchangés |

Le chemin de l'annulation évite `/inducements/cancel`, qui chevaucherait
`/inducements/{team_id}`. Le bouton est un `hx-post` : le middleware CSRF exige
`HX-Request: true`.

## 5. Conséquences sur les données de production

| Carte | Données modifiées | Effet sur l'existant |
|---|---|---|
| 626 — Annuler | un événement **ajouté**, aucun modifié ; aucune migration | les anciens flux se rejouent tels quels ; **aucun mouvement d'argent** — la trésorerie n'est débitée qu'à la publication ; les comptes rendus en `PreMatch` sans action deviennent annulables ; les journaliers retirés sortent de `players` par le chemin existant |
| 627 — Achats figés | aucune : « a validé » se reconstruit au rejeu | les comptes rendus en cours dont une équipe a validé sont **figés pour elle dès la livraison**, y compris en `ReadyToPublish` et après dépublication ; un outsider qui a acheté avant le favori garde ses achats ; les deux défauts du § 1 ne peuvent plus se produire, ceux qui se seraient déjà produits restent en l'état |
| 628 — Récapitulatif | aucune (deux champs `recorded_by` au rejeu, décision 5) | affichage ; les rapports où les deux équipes ont validé montrent le récapitulatif dès la livraison |
| 629 — Panier | aucune | affichage ; le formulaire envoyé ne change pas |
| 630 — E2E | aucune | — |

**Perte de capacité acceptée** (2026-11-01) : un compte rendu dépublié, ou
arrivé à l'étape 5, ne propose plus l'annulation — R3 l'exclut par l'état.
Une erreur de coups de pouce découverte après l'étape 5 ne se corrige plus,
**même sur un match sans aucune action**.

**Mesure facultative avant livraison**, en lecture seule, pour savoir combien de
rapports seront figés : compter dans `match_report_proj` les rapports en
`PreMatch` ou `ReadyToPublish` dont `home_inducements` ou `away_inducements`
n'est pas `NULL`.

## 6. Les tests

- **Unitaires (domaine)** : R1 — `Passer` vaut validation ; R2 — racheter après
  validation est refusé, y compris depuis un `ReadyToPublish` ; R3 — annulation
  refusée avec une action, permise après sa suppression, refusée en
  `ReadyToPublish` ; R4 ; R5 — après annulation, `petty_cash` vaut l'écart de
  valeur et le budget de l'outsider baisse ; rejeu de `InducementsCancelled`,
  et sa séquence invalide hors `PreMatch`.
- **Unitaires (use case)** : l'annulation appende `InducementsCancelled` puis un
  `TempPlayersReset` par camp qui a des joueurs temporaires, émet les trois,
  et le reset nomme les journaliers ; conflit → `Conflict`.
- **E2E** (carte 630) : panier — retirer un article commun sans ouvrir son
  onglet ; achats figés — un second POST répond 409 ; annulation — le
  récapitulatif apparaît, l'annulation se confirme dans la page, rouvre le
  choix du favori, et l'outsider rachète avec un budget recalculé ; plus de
  bouton après la première action.
- **Non-régression des e2e existants** — lus un par un :

| Fichier | Revient-il modifier des achats ? | Effet |
|---|---|---|
| `test_match_report_inducements.py` | non : un POST par équipe et par rapport | `test_cart_footer_visible` (`:262`) rouvre la page du favori **après** que `test_topdog_pass_redirects_to_underdog` (`:180`) l'a fait passer, sur le même rapport (fixture de module) : avec R7 il atterrit chez l'outsider, pied de page présent — **passe, mais par accident** ; à rendre indépendant. `test_underdog_pass_redirects_to_step3` (`:208`) change d'attente si décision 4 est retenu |
| `test_inducement_treasury.py` | non : favori puis outsider, une fois chacun ; dépublie sans racheter | aucun |
| `test_inducements_restreints.py` | non : le premier achat est refusé en 422 (aucun événement), le second vise l'autre équipe | **R9** : il fait passer le favori d'abord si l'équipe visée est l'outsider (627) |
| `test_inducements_tarif_reduit.py` | non : un seul achat, une seule équipe | **R9** : idem (627) |
| `test_cuistot_halfling.py` | non : un seul achat, une seule équipe | **R9** : idem (627) |
| `test_match_report_step2_mercenaires.py` | non : `test_submit_with_mercenaire…` (`:266`) valide le favori ; les tests suivants ouvrent d'autres rapports | sélecteurs du panier (`.mr-cart-merco-item`, `.mr-cart-merco-remove`) à garder ou à reprendre en 629 |
| `match_report_helpers.ensure_inducements` et ses copies (`test_haine_journalier`, `test_match_report_recap`, `test_match_report_step5`, `test_player_detail`, `test_player_spp_spending`, `test_team_detail_state_banner`, `test_match_report_step3_4_actions`) | suivent les redirections, trois POST au plus, sans assertion sur le statut | un troisième POST refusé en 409 arrête la boucle — sans effet |
| `visual/urls.py` (`rapport-inducements`) | lecture | la capture montrera le récapitulatif si le rapport de démonstration a ses deux équipes validées |

- `tests/impact-map.toml` : aucune entrée nouvelle si les scénarios de 630
  vivent dans `test_match_report_inducements.py` ; sinon, le nouveau fichier y
  entre avec `match_report`, `references`, `teams`, `players`.
