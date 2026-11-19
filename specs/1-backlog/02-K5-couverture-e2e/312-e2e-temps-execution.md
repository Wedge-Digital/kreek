# Suite e2e — réduire le temps d'exécution

**Priorité : moyenne**
**Contexte :** `tests/e2e/` — fixtures partagées

## Le constat

La suite prend ~7 min 30 pour ses tests, et **30 fichiers sur 38** construisent
une compétition complète avant d'assurer quoi que ce soit. Le coût n'est pas
dans les assertions : il est dans les fixtures.

Le projet a déjà fait ce constat deux fois et l'a résolu deux fois. La
docstring de `competition_lifecycle.py` le dit :

> `build_and_submit_team_http` appelle les mêmes routes en HTTP direct. […]
> rejouer le parcours au clic n'y teste rien de plus et coûtait ~2,4 s par
> équipe (dont 1,65 s de `wait_for_timeout` fixes) contre quelques dizaines de
> millisecondes.

Les rapports de match ont suivi le même chemin avec `match_report_helpers`.

## Piste 1 — la dernière fixture restée au clic

`create_full_competition` pilote encore **18 étapes de navigateur**, payées par
les 30 fichiers qui l'utilisent. Même motif, même endroit, précédent déjà
mesuré.

Condition non négociable : **un test doit conserver le parcours réel**. C'est
déjà le cas — `test_competition_full_lifecycle` et
`test_full_competition_creation_flow` exercent la création au clic. Le
basculement ne doit pas les toucher.

Le gain n'est pas chronométré à ce stade : le motif est établi, le chiffre non.
À mesurer avant/après plutôt qu'à annoncer.

- [ ] Variante HTTP de `create_full_competition`
- [ ] `build_full_competition` l'utilise
- [ ] Les deux tests de parcours réel restent au clic, vérifié explicitement
- [ ] Mesure avant/après consignée dans la carte

## Piste 2 — les tests qui ne regardent aucun pixel

Certains tests e2e n'ouvrent jamais de page : ils pilotent en HTTP et
n'assurent que sur les projections.

`test_spp_scale.py` en est l'exemple net — publication en HTTP, assertions sur
`players_proj`, aucun rendu vérifié. Sa place n'est pas Playwright.

**Destination : la tier Rust qui existe déjà**, pas le harnais handler de la
carte 311. Le patron est
`src/app/ranking/io/app_events/tests/test_match_report_published_pipeline.rs` —
un `#[sqlx::test]` de pipeline d'app event. Aucun outillage à construire.

Le critère de tri n'est pas « lent » mais : **le rendu fait-il partie de ce
qu'on affirme ?** `test_detailed_standings.py` vérifie les colonnes décisives
et leurs classes CSS — il reste en e2e sans discussion.

- [ ] Passer en revue les 38 fichiers avec ce critère
- [ ] Migrer les candidats un par un, jamais en lot
- [ ] Retirer chaque test migré de `tests/impact-map.toml` dans le même commit
      (axe 8 de `check-arch` refuse une entrée orpheline)

## Ce que cette carte ne fait pas

**Elle ne touche pas au partage des fixtures.** `build_full_competition`
construit une compétition **dédiée par fichier**, délibérément. Élargir la
portée pour gagner du temps rouvrirait l'isolation — or la flakiness de la
suite avait justement été tracée à une saturation de la base sur l'espace
partagé. Le temps gagné se paierait en tests instables.

**Elle ne dépend pas de la carte 311.** Le harnais handler n'allège rien de
l'existant : les contrats HTTP ne sont aujourd'hui pas testés du tout. Les deux
cartes répondent à deux besoins distincts — celle-ci raccourcit ce qui existe,
l'autre empêche ce qui viendra de s'ajouter au mauvais étage.

## Bénéfice attendu au-delà du chronomètre

Des fixtures plus rapides réduisent la pression sur la base, à laquelle la
flakiness observée avait été tracée.

**Correction, carte 317** : cette attribution était fausse. La flakiness vient
de **transactions fantômes** — une connexion laissée `idle in transaction` par
l'annulation d'une requête, qui bloque les suivantes pendant des minutes. La
charge n'est que le déclencheur, pas la cause.

La 317 est donc prioritaire sur celle-ci.

**Second retournement, à la clôture de la 317** : il n'y a pas de handler
fautif à corriger — `sqlx` ne garantit pas l'envoi du `ROLLBACK` quand un future
est annulé, c'est une propriété de la bibliothèque, et le filet posé côté base
est le traitement définitif.

Du coup **cette carte n'est plus le symptôme, elle est le levier restant** :
moins de requêtes lentes, moins de timeouts clients, donc moins d'annulations et
moins de fuites. Elle passe de « confort » à « seule action encore utile sur la
flakiness ».
