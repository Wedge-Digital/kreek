# Les tests du temps réel

**Épic :** E23 — Le temps réel : connexions, présence et notifications · **Ordre :** 4 · **Dépend de :** 642 à 646
**Priorité :** normale
**Conception :** `docs/specs/saisie-mobile/E23-cadrage.md` (§ 8, § 10, § 11.17)

## Objectif

Le « Terminé quand » de l'épic, rejoué en navigateur : trois matchs sont saisis
en parallèle dans le même espace ; chaque coach ne reçoit, en toast, que les
notifications de ses propres matchs ; le journal d'un match se met à jour chez
qui le regarde sans recharger ; un coach qui ferme son onglet disparaît du
registre.

## Ce que le code fait

Chaque carte de l'épic porte déjà ses tests unitaires et e2e, chacun sur un
match : canal (642), faits sortants (643), présence (644), notifications (645),
direct (646). Aucun ne voit plusieurs matchs à la fois, ni l'étanchéité entre
eux.

- Le serveur e2e ne connaît par en-tête que deux identités, `DevCoach` et
  `E2E Coach 01` (`src/web/middleware/bypass_auth.rs:36-51`,
  `tests/e2e/test_match_report_solo_entry.py:307-308`).
- `bypass_auth` ne s'applique qu'aux routes protégées et qu'à une session
  vide : un contexte qui passe d'abord par `/auth/login` garde l'identité qu'il
  y prend. Le seed crée onze coachs supplémentaires au mot de passe
  `SEED_PASSWORD` (`src/cli/seed_e2e.rs:37`, `:47`).
- `build_full_competition` distribue les équipes par ordre de nom de coach
  (`tests/e2e/competition_lifecycle.py:583-592`) : six équipes vont à DevCoach
  et à `E2E Coach 01` à `05`.

## Ce qu'il faut faire

- **`tests/e2e/test_realtime_three_matches.py`** : une compétition à six
  équipes, trois rencontres d'une même journée qui n'ont aucun coach en commun.
  Six contextes de navigateur, un par coach — DevCoach, le membre simple par
  en-tête, les quatre autres par `/auth/login`.
- **Les trois matchs saisis en parallèle** : sur chacun, un coach saisit des
  actions ; l'autre, sur une page quelconque de l'application, reçoit le toast
  de son match et de lui seul. Les quatre autres n'en reçoivent aucun de ce
  match — constaté **après** que le destinataire a reçu le sien.
- **Le direct** : sur un des trois, l'autre coach suit `/live` ; son journal se
  met à jour à chaque action, et ne bouge pas quand on saisit sur les deux
  autres.
- **La sortie du registre** : un coach ferme son onglet ; le widget de
  présence de son adversaire perd sa mention.
- **Un helper** `tests/e2e/realtime_helpers.py` : connexion d'un coach seedé,
  attente d'un toast par son texte, témoin positif avant tout constat
  d'absence. Les copies que les cartes précédentes auraient laissées y sont
  ramenées.
- **Les pièges à éviter** (§ 8) : pas de `networkidle` ; le toast de succès
  disparaît en 2,5 s, celui à lien en 6 s ; en debug, chaque onglet tient aussi
  le flux de `tower-livereload` — vérifier que six contextes ne touchent pas la
  limite de six connexions HTTP/1.1, chacun ayant son propre pool.

## Fichiers touchés

`tests/e2e/test_realtime_three_matches.py`, `tests/e2e/realtime_helpers.py`,
les tests e2e des cartes 642 à 646 si leurs helpers y sont ramenés,
`tests/impact-map.toml`.

## Tests

- **E2E** : le scénario ci-dessus, en ordinateur.
- **Unitaires** : aucun de nouveau — l'étanchéité entre trois matchs est déjà
  éprouvée par le routeur (645) ; cette carte la constate en navigateur.
  Écart relevé avec la fiche (§ 8, « trois matchs simultanés restent
  étanches ») : l'unitaire du routeur en éprouve **deux**
  (`two_simultaneous_matches_stay_sealed`, `test_notification_router.rs`).
  Deux suffisent à la logique — un troisième match n'ajoute aucun cas —, et
  l'e2e en joue trois.

## Ce qui a été fait

- **Chaque match est démarré par son coach domicile** (DevCoach, E2E Coach 02,
  E2E Coach 04), par `create_draft`, sous son identité : `POST
  /match-report/new` démarre le rapport au nom de qui l'envoie. Démarré par
  DevCoach, admin qui ne coache pas, M2 aurait eu deux saisisseurs, et `/live`
  ne montre la présence qu'à qui ne saisit pas (`write.is_none()`). La fixture
  le vérifie : le démarreur lu en base doit être le coach domicile.
  Première version fausse : le `via` passé à `ensure_pre_match` n'agissait
  jamais, le rapport étant déjà démarré par `create_draft` sous DevCoach —
  T2 échouait sur un widget de présence absent.
- **Le match d'un toast se lit à son lien** : le journal de la 645, déplacé
  tel quel, puis étendu à `{ text, href }`.
- **L'ordre des coachs est vérifié en base** dans la fixture, pas supposé.
- `realtime_helpers.py` reçoit les copies des cartes 642, 644, 645 et 646 ;
  `match_report_helpers.py` reçoit `start_match` (ex-`_started` de la 646) et
  un paramètre `via` sur `create_draft`, `ensure_inducements` et
  `record_action_api`.
- `test_auth_coach_name_casse.py` garde sa propre connexion : ce n'est pas une
  copie laissée par l'épic.

## Hors périmètre

- Une identité de contournement supplémentaire : écartée (décision 17).
- Les cas de panne (redéploiement, `Lagged`) : éprouvés par les unitaires de la
  642 et de la 645.

## Conséquences sur les données de production

Aucune.

## Terminé quand

- [ ] Trois matchs saisis en parallèle : chaque coach ne reçoit que les toasts de son match
- [ ] Le journal d'un match se met à jour chez qui le regarde, et pas sur les saisies des deux autres
- [ ] Un coach qui ferme son onglet disparaît de la présence de son adversaire
- [ ] `tests/impact-map.toml` mis à jour
- [ ] `make e2e` vert
