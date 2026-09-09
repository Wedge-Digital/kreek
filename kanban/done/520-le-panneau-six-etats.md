# Le panneau, six états

**Priorité : haute — c'est l'écran**
**Épic :** E16 — Sondage de présence
**Dépend de :** 519, et les use cases 515 à 518 pour ce qu'il affiche
**Fichiers :** `src/app/competitions/io/web/admin/presences_widgets.rs`,
`.../templates/admin/widgets/presences-panel-*.html`,
`src/app/competitions/routes.rs`, `src/app/competitions/router.rs`

## Cette carte déclare et branche sa route

La 519 n'a posé que les trois routes qu'elle sert — `presences`,
`presences/rounds`, `presences/panel`. **Les neuf autres appartiennent aux cartes
qui les servent.** Une route déclarée sans handler ne compile pas, et une route
branchée sur un handler vide est une porte ouverte sans garde : neuf handlers
factices passeraient `check-arch` et l'axe 4 sans que rien ne les appelle.

`presences/panel` existe déjà — cette carte en remplit le handler. Elle n'a donc
pas de route à ajouter, mais elle hérite de la garde : `require_admin_access`
puis `journee_de_la_saison`, **sur le fragment aussi**, `space_scope` n'ayant pas
de résolveur pour `round_id` (carte 416).

## L'objectif

Le panneau rend **l'un des six fragments** — aucun sondage, en cours, clos,
tirage proposé, journée appariée, défection — et **c'est le serveur qui
choisit**.

Un widget par état obligerait le navigateur à savoir où en est la campagne pour
demander le bon endpoint : la machine à états serait écrite deux fois, une fois
dans le domaine et une fois en JavaScript, et c'est la seconde qui dériverait.

## Le choix est une fonction, pas une suite de `if`

```rust
enum Panneau { Aucun, EnCours, Clos, Tirage, Appariee, Defection }

fn etat_du_panneau(survey: Option<&PresenceSurvey>, journee: &EtatJournee, maintenant: ...) -> Panneau
```

Elle se nourrit de `statut()`, des appariements réels de la journée et de
`desaccord(...)` — trois questions déjà répondues par le domaine. L'écrire en
`if` dans le handler l'aurait dupliquée entre le GET du panneau et les neuf
actions qui rendent un refus, et c'est la seconde copie qui aurait dérivé.

## Cinq gabarits pour six états

« En cours » et « clos » partagent le leur, comme la maquette qui les rend avec
le même markup et deux bandeaux.

**Six structs `Template`, pas un gabarit à branches.** Le `{% include %}`
d'Askama n'accepte qu'un chemin littéral : un fichier unique aurait voulu dire
six `{% if %}` imbriqués. Le handler choisit la struct et rend `Html<String>` —
`admin_page.rs` procède déjà ainsi.

## Les VM ne dérivent rien

`AvancementVm` reçoit **quatre comptes du domaine** — `compte_presents()`,
`compte_absents()`, `compte_sans_reponse()`, `engagees()`. Le réflexe serait de
faire `rows.len()` sur chaque colonne : c'est exactement le défaut de la carte
495, où la vue recomptait ce que le domaine savait compter, et comptait autre
chose.

Même raison pour `DrawVm::inedites` et `revanches` : ils viennent du domaine, qui
a produit les `Historique`. Les recompter marcherait aujourd'hui et deviendrait
faux le jour où une rencontre porterait un troisième motif.

`DrawRowVm::tag` — « 2e rencontre · J1 » — vient de `Historique`, jamais d'une
relecture des journées par le VM. **Le tirage sait pourquoi il a concédé ; il le
dit.**

## Ce qui ne va pas dans le panneau

Pas de menu au « ⋯ » : la correction manuelle d'une réponse est **deux boutons
`présent` / `absent` posés sur la carte**, un POST par bouton. Le menu aurait
demandé un `x-data` par carte et un calque qu'un parent en `overflow` peut
rogner, pour ajouter un clic à chaque correction.

Aucun Alpine, aucun état dupliqué.

## Checklist

- [x] `etat_du_panneau` — **cinq** variantes, cf. ci-dessous — et les six structs
      `Template`
- [x] Six gabarits + trois fragments partagés, `hx-disinherit="*"` sur chaque racine
- [x] `RoundHeadVm`, `DestinatairesVm`, `AvancementVm`, `AnswerRowVm`, `DrawVm`,
      `DrawRowVm`, `DefectionVm` — tous avec `from_domain()` co-localisé
- [x] Le badge « saisi par vous » (R6), « sans adresse connue » (R3), le motif de
      R15, les écartées de R18, l'optimum non prouvé de R8
- [x] Le CSS des cinq états dans la feuille de la 519
- [x] 13 tests unitaires · 1 e2e · `make lint`, `make check-arch`,
      `make test` — 1876/1876 · `make e2e` — 376/376

## Ce que la réalisation a corrigé

**`Panneau` a cinq variantes, pas six.** `Tirage` n'est pas calculable : l'aperçu
ne persiste rien — la réponse *est* le fragment, et un rechargement revient au
sondage clos. Rien dans `(campagne, journée, aujourd'hui)` ne peut dire qu'un
tirage vient d'être proposé, et le garder dans l'enum créerait une branche
inatteignable dans le `match` du GET : le genre de code que personne n'ose
supprimer parce qu'il a l'air prévu.

Le gabarit du tirage et son `DrawVm` sont bien livrés ici — cette carte possède les
six vues — mais rendus par `rendre_tirage`, publique, que l'action `draw` de la
carte 521 appellera. Elle n'aura qu'à câbler.

**L'ordre des branches est la règle** : une défection prime sur « appariée », qui
prime sur le statut. Un désaccord non traité est ce que l'organisateur doit voir
en premier ; l'annoncer « appariée » lui cacherait le travail qui reste.

## Trois décisions prises en écrivant

**Pas de barre segmentée**, alors que la maquette en montre une. Ses proportions
demanderaient un `style="width: …%"` par segment, et les styles inline sont
interdits. Trois segments de largeur égale mentiraient sur les proportions — pire
qu'aucune barre. Cinq gabarits livrés portent des `style=""` ; ce n'est pas une
raison d'en ajouter un. Une barre honnête coûterait vingt classes de largeur ou du
JS, et les quatre comptes disent déjà tout.

**Le panneau « appariée » n'affiche pas d'étiquette de rencontre.** Les
appariements écrits ne portent pas leur `Historique` : le relire depuis les
journées serait faire dériver au VM une valeur que le tirage avait produite. Le
`tag` est vide, et le gabarit ne rend alors pas le `<span>`.

**Les noms viennent du roster, jamais un identifiant brut.** `journee_appariee`
traduit chaque `TeamId`, avec « Équipe désengagée » en repli — c'est le défaut de
la carte 506, un identifiant de vingt-six caractères là où on attend un nom.

## Les initiales, reportées de la 514

Deux lettres du nom d'équipe, mots-liens sautés : « Les Crocs du Chaos » → `CC`,
là où `initials_from` de `teams` donnerait `LC`. **Les accents sont gardés** —
« Étoiles de Naggaroth » → `ÉN` ; la maquette affiche `EN` mais `GÉ` ailleurs,
une incohérence d'écriture à la main, et rien ne justifie de retirer un accent
que le nom porte. Limite assumée : « FC Barcelone » → `FB`.

## Les gabarits sont muets, et c'est la découpe

Aucun bouton : les neuf routes appartiennent à la 521, et un `hx-post` vers une
route inexistante ne compile pas côté Askama. Le panneau est **complet en lecture,
muet en écriture** — la 521 lui ajoute ses commandes en même temps que ses
routes.
