# Le panneau, six états

**Priorité : haute — c'est l'écran**
**Épic :** E16 — Sondage de présence
**Dépend de :** 519, et les use cases 515 à 518 pour ce qu'il affiche
**Fichiers :** `src/app/competitions/io/web/admin/presences_widgets.rs`,
`.../templates/admin/widgets/presences-panel-*.html`

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

- [ ] `etat_du_panneau` et les six structs `Template`
- [ ] Les cinq gabarits, `hx-disinherit="*"` sur la racine
- [ ] `DestinatairesVm`, `AvancementVm`, `AnswerRowVm`, `DrawVm`, `DrawRowVm`,
      `RoundHeadVm` — tous avec `from_domain()` co-localisé
- [ ] Le badge « saisi par vous » (R6), la mention « sans adresse connue » (R3),
      le motif de R15 sous le bouton inactif, les écartées de R18
- [ ] Le CSS des six états dans la feuille de la 519
- [ ] `make lint`, `make check-arch`, `make test`
