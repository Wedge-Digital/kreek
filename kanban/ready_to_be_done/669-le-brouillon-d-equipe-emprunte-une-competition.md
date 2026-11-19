# Le brouillon d'équipe emprunte une compétition

**Épic :** aucune — stabilité de la suite e2e · **Trouvée à :** la carte 641

## Le constat

`test_draft_team_errors.py::test_submit_without_coach_shows_error`, lancé seul
juste après `make reset_e2e_db`, échoue : `_select_competition_and_season`
attend une vraie option dans le sélecteur de compétition
(`.ks-option:not(.ks-empty)`), et la base fraîche n'en a aucune — le seed e2e
ne crée pas de compétition.

Dans la suite complète, il passe parce que les fichiers `test_competition_*`
tournent avant lui (ordre alphabétique) et laissent leurs compétitions en base.
**Le test emprunte un décor qu'il ne pose pas.** La base accumulée masquait la
dépendance ; la remise à zéro de la 641 la rend visible, mais elle ne la crée
pas.

Elle cassera à coup sûr :

- quand un fichier est relancé seul pour diagnostiquer (`pytest
  test_draft_team_errors.py`) — c'est déjà le cas ;
- avec le parallélisme (`pytest-xdist`), où l'ordre entre fichiers n'existe
  plus.

`test_submit_without_competition_shows_error` et `test_error_has_correct_style`
ne choisissent pas de compétition et ne sont pas concernés.

## Ce qui change

Le test pose son propre décor : une fixture de module crée **une** compétition
publiée par `competition_lifecycle.create_full_competition(page,
competition_create_url, num_rounds=1)` — le même appel que
`test_build_and_finalize_team.py` —, et `_select_competition_and_season`
choisit **celle-là**, par son nom, plutôt que la première venue.

Choisir par le nom compte autant que créer : « la première option » reste une
dépendance au contenu de la base, même quand le décor existe.

## Ce que la carte ne couvre pas

- **Chercher les autres emprunteurs** : d'autres tests prennent peut-être
  « la première compétition » ou « la première équipe » sans la créer. Un
  passage de chaque fichier seul, sur base fraîche, les trouverait ; c'est le
  travail de la carte du parallélisme, qui devra le faire de toute façon.
- **Un seed qui crée une compétition** : écarté. Il rendrait le test vert sans
  le rendre autonome, et chaque test qui en dépendrait partagerait son état.

## Fichiers touchés

`tests/e2e/test_draft_team_errors.py` ; `tests/impact-map.toml` inchangé (le
test déclare déjà `competitions`).

## Tests

C'est la carte elle-même : le fichier passe **seul**, sur base fraîche, et
toujours dans la suite complète.

## Conséquences sur les données de production

Aucune : seul un test e2e change, et la base e2e locale reçoit une compétition
de plus par exécution, vidée au `make e2e` suivant (carte 641).

## Terminé quand

- [ ] `make reset_e2e_db` puis `uv run pytest test_draft_team_errors.py`
      (depuis `tests/e2e`) : 3 réussis, trois fois de suite ;
- [ ] la suite complète reste à 544 réussis.
