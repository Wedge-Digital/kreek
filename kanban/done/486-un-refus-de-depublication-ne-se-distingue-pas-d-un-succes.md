# Un refus de dépublication ne se distingue pas d'un succès

**Priorité : haute** — la CI de `demo` est rouge sans rien signaler du code
**Dépend de :** rien · **Sans épic**
**Trouvée par :** la CI, trois runs sur quatre

## Le symptôme

```
FAILED test_match_report_correction.py::test_deux_corrections_successives_aboutissent
AssertionError: correction 1 — non satisfait après 30s (dernier : False)
```

Un seul test sur 341. Les trois autres jobs — qualité, tests unitaires, audit —
sont verts. Le même test échoue par intermittence depuis plusieurs runs, et
jamais en local.

**Le message accuse le délai. La cause peut être un refus arrivé en 6 ms.**

## Ce qui est mesuré

Le parcours réel, joué localement : publier, faire valider la phase
d'amélioration du camp domicile par l'API, puis dépublier.

```
unpublish -> HTTP 200 en 0.006s, corps 0o, en-têtes htmx {'hx-refresh': 'true'}
phase du rapport 2 s après : Published        ← refusé
```

Et le même appel quand il aboutit :

```
unpublish -> HTTP 200, corps 0o, en-têtes htmx {'hx-refresh': 'true'}
phase du rapport 2 s après : ReadyToPublish   ← accepté
```

**Rigoureusement identique.** `unpublish_response` fait passer `Ok(())` et
`Err(NotEligible(_))` par la même sortie :

```rust
Ok(()) | Err(UnpublishMatchReportError::NotEligible(_)) => refresh(),
```

Le choix est délibéré et se défend à l'écran — la page se recharge et montre le
motif recalculé. Mais **aucun appelant non humain ne peut distinguer les deux
cas**, et le test n'a rien à lire.

Le voisin immédiat, lui, contrôle : `publish()` vérifie `302/303` et échoue
aussitôt. Il le peut parce que sa route redirige.

## Rien n'est journalisé

Dans `recap_controller.rs`, seul `Repository(e)` produit une ligne. Un refus
d'éligibilité ne laisse **aucune trace** : ni dans `docker logs`, ni dans la
sortie de la CI. C'est pourquoi le journal du run n'en dit pas plus que le test,
et pourquoi cette enquête a demandé de reproduire le refus à la main au lieu de
le lire.

## La cause probable, que je n'ai pas reproduite

`is_team_in_player_improvement` rend `false` dans **deux situations opposées** :

- la phase est **dépassée** — le coach a validé ses améliorations ;
- la phase n'est **pas encore arrivée** — l'app event ne l'a pas encore fait
  entrer en `PlayerImprovement`.

Or cette entrée vient d'un app event **cross-BC**, asynchrone par construction
(`teams/io/app_events/match_report_published_listener.rs`). Le test, lui, attend
`match_report_proj.phase = 'Published'` — une projection **intra-BC**, écrite
dans la même transaction que l'événement, donc vraie **avant** que `teams` ait
réagi.

Mesuré localement : l'app event gagne la course en **moins de 40 ms, 3 fois sur
3**. Jamais de refus ici. En CI chargée, il peut perdre — et je ne peux pas le
prouver sans le journal du serveur de CI, qui n'existe pas.

Le bloqueur s'appelle `PhaseAdvanced` dans les deux cas, et le message affiché
serait alors *« X a validé sa phase d'amélioration »* — faux : l'équipe n'a rien
validé, elle n'y est pas encore entrée.

## Ce que la carte corrige

### 1. Le refus laisse une trace

Une ligne de journal nommant le bloqueur, à l'endroit où le refus est décidé.
Sans elle, ni la CI ni la production ne sont diagnosticables — et la prochaine
occurrence coûtera la même enquête.

C'est le point qui compte le plus : il ne change aucun comportement et rend
observable une décision qui ne l'est pas.

### 2. `_unpublish` constate au lieu d'attendre

Le test attend la transition ; si elle ne vient pas, il **lit le motif** sur
l'écran de récapitulatif — qui le porte déjà — et échoue en le citant. Quatre
sites d'appel dans le fichier, tous à reprendre : aucun ne lit sa réponse
aujourd'hui.

Le message doit dire *« dépublication refusée : Tueurs de Nains a validé sa
phase d'amélioration »*, pas *« non satisfait après 30 s »*.

### 3. Le test attend l'entrée en `PlayerImprovement`

Avant de dépublier, comme le test voisin attend déjà ses compensations
(`_wait(_team_phase(home) == "MatchReporting")`). C'est la course elle-même,
côté test.

## Ce que la carte ne fait pas

**Elle ne touche pas au produit sur la course.** La fenêtre mesurée est de
40 ms ; aucun commissaire ne dépublie un rapport dans les 40 ms qui suivent sa
publication. Distinguer « pas encore arrivée » de « dépassée » dans
`is_team_in_player_improvement` corrigerait un message trompeur dans une
situation que seul un test atteint. **Si le message compte, c'est une carte à
part** — pas un ajout silencieux à celle-ci.

**Elle ne change pas la sortie HTTP de l'`unpublish`.** Rendre un code distinct
au refus casserait le rechargement qui montre le motif, et ce comportement est
motivé dans le code. Le journal donne l'observabilité sans y toucher.

**Elle ne traite pas `test_match_reporting_banner_resume_link_navigates`**, cité
dans un run antérieur. Sa nature est différente : il charge la page une fois et
attend un bandeau produit par un app event cross-BC, sans jamais recharger. Si
l'instabilité persiste après cette carte, elle mérite la sienne.

## Ce qui a été fait, et ce que ça a appris

### Le helper de capture a été extrait, pas recopié

`panic_response.rs` (carte 349) portait déjà `Capture`, `Champs` et
`sous_le_filtre_de_production` — 35 lignes qui posent **le filtre réellement
construit au démarrage**, et non une chaîne recopiée. Les copier une deuxième
fois aurait garanti la troisième. Ils vivent désormais dans
`common/services/observability/capture_journal.rs`, et `panic_response.rs`
pointe dessus : ses quatre tests, inchangés, valident la copie.

Une variante était nécessaire : `with_default` prend une fermeture **synchrone**
et ne peut pas envelopper un `.await`. D'où `capture_sous_le_filtre_de_production`,
qui rend un garde — la forme utilisable depuis un `#[tokio::test]`.

### Deux falsifications qui n'en étaient pas

**Le filtre de `cargo test` ne prend pas de regex.** `cargo test "journalis\|ne_journalise"`
n'a sélectionné aucun test — « 0 passed, 1609 filtered out » — et les quatre
mutations sont passées pour concluantes alors qu'aucun test n'avait tourné.

**`error: test failed` n'est pas une erreur de compilation.** Le détecteur
suivant cherchait `^error` pour écarter les mutations qui ne compilent pas ; il
attrapait la ligne finale de `cargo test`, et rapportait « ne compile pas » pour
des mutations qui faisaient parfaitement échouer le test. Corrigé en cherchant
`^error[` et `^error: could not compile`.

Deux fois de suite, l'instrument de mesure a menti dans le sens rassurant. C'est
la même famille d'erreur que celle que la carte corrige.

### La première attente masquait le motif

`depublier` commence par attendre que le rapport devienne corrigeable. Écrite
d'abord avec le `_wait` ordinaire, cette attente-là rendait *exactement* le
message qu'on remplaçait — « non satisfait après 30 s » — pour le cas le plus
intéressant : un rapport définitivement bloqué, qui n'atteint jamais la
dépublication. Les **deux** attentes citent maintenant le motif.

### Le message, avant et après

```
✗ correction 1 — non satisfait après 30s (dernier : False)

✓ correction 1 — le rapport n'est jamais devenu corrigeable, après 30 s.
  Le récapitulatif dit : « Granitiers a validé sa phase d'amélioration.
  Le rapport n'est plus corrigeable. »
```

## Un incident non expliqué, à ne pas confondre avec celui-ci

Le premier `make e2e` complet a échoué sur `test_accueil_derniers_resultats`,
un `Page.goto` en délai dépassé — puis le test est passé seul, et la suite
complète relancée est passée entière.

Hypothèse **non vérifiée** : `cargo-watch` surveille le dépôt, les artefacts de
Playwright s'y écrivent, et le serveur redémarre au milieu du run. Ça ne
concerne que le poste de développement — il n'y a pas d'observateur en CI. Si le
symptôme revient, c'est sa carte.

## Tests

| Test | Ce qu'il prouve |
|---|---|
| `un_refus_d_eligibilite_est_journalise` | la ligne existe, avec le bloqueur |
| `une_depublication_reussie_ne_journalise_pas_de_refus` | et son contraire |
| `test_deux_corrections_successives_aboutissent` | inchangé — c'est lui qu'on stabilise |
| une falsification manuelle | phase forcée par l'API : le test doit échouer **en citant le motif**, pas au bout de 30 s |

La falsification est ici la vraie preuve : le test corrigé doit rendre un
message utile face au refus que j'ai su reproduire.

## Checklist

- [x] Le refus d'éligibilité journalise son bloqueur
- [x] Les deux tests unitaires du journal, falsifiés quatre fois (ligne retirée, niveau `debug`, cible hors `kreek::`, motif absent)
- [x] `depublier` constate, et cite le motif lu à l'écran — aux **deux** attentes
- [x] Les quatre sites d'appel du fichier passent par lui
- [x] L'attente d'un rapport *corrigeable* avant chaque dépublication — plus général que la phase d'une équipe, et sans connaître les deux camps
- [x] Falsification : phase validée par l'API → le message nomme la cause
- [x] `make lint`, `make test` (1608), `make check-arch` (16 axes), `make e2e` (341, 0 échec)
- [ ] La CI de `demo` verte sur deux runs consécutifs
