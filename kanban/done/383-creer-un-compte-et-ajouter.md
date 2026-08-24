# Créer un compte et ajouter

**Priorité : haute** — second chemin de l'onglet
**Dépend de :** 380 et 382
**Conception :** `docs/specs/space-admin/ajout-direct/{03-back.md, 04-dtos.md}`
**Fichiers :** `src/app/spaces/io/web/host_layout.rs`,
`src/infrastructure/spaces/host_layout_adapter.rs`, la page hôte

## Objectif

Afficher le widget de création de compte d'`auth` dans l'onglet, et transformer
son succès en appartenance.

```
[widget auth] créer le compte  ──►  accountCreated { coach_id, name }
                                          │
[spaces] POST …/members/add ◄─────────────┘  + le profil choisi
```

## L'injection

```rust
// dans ISpacesHostLayout, aux côtés d'upload_widget()
fn coach_creation_widget(&self, prefill: CoachPrefill<'_>) -> String;
```

L'hôte rend le fragment d'`auth` et le donne à `spaces` sous forme de chaîne.
`spaces` **ne connaît pas `auth`** : il place le fragment et écoute un événement
DOM.

`src/infrastructure/spaces/host_layout_adapter.rs` est le seul point du projet
qui relie les deux — c'est déjà sa fonction pour le layout et le widget d'upload.

## Le sélecteur de profil reste à `spaces`

`SpaceProfile` est son concept. La grille Pseudo · Email · Profil de la maquette
passe donc sous **deux propriétaires** : les deux premiers champs et le bouton
viennent d'`auth`, le troisième de `spaces`, posé à côté du fragment injecté.

**C'est une contrainte sur le dessin, à régler à la maquette avant de coder.**
Les deux moitiés doivent se lire comme une seule ligne.

## Le contrat que rien ne vérifie

Le nom `accountCreated` et les clés `coach_id` et `name` franchissent la
frontière entre deux BCs **par le navigateur**. Ni le compilateur, ni
`cargo test`, ni `check-arch` ne les voient — c'est un `grep`, et le `CLAUDE.md`
le dit lui-même : il ne voit ni les chaînes littérales ni les attributs HTML.

**Seule la carte 384 le vérifie.** À commenter des deux côtés — ici et dans la
carte 380.

## Deux allers-retours, et l'échec du second

Si la création réussit et que l'ajout échoue, **le compte existe et
l'appartenance non**.

Le message le dit explicitement — « le compte a bien été créé, mais l'ajout à
l'espace a échoué ; retrouvez le coach dans la recherche ci-dessus » — plutôt
que de rester générique. Un compte orphelin dont l'administrateur ignore
l'existence, c'est un pseudo et un email pris, et une seconde tentative qui
échouera sur `PseudoDejaPris` sans qu'il comprenne pourquoi.

La reprise ne demande rien de neuf : le coach apparaît dans le panneau de
recherche, où un clic l'ajoute.

## Le pré-remplissage

Quand la recherche ne rend rien, ce qui a été tapé part dans le champ Pseudo ou
Email selon qu'il contient un `@`. **La répartition est faite par `spaces`**, qui
sait ce que l'utilisateur cherchait, et passe par `CoachPrefill`.

## Checklist

- [x] `coach_creation_widget()` ajoutée à `ISpacesHostLayout`, avec son
      `CoachPrefill` **défini par ce BC** — il ne peut pas importer les types de
      celui qui rend le formulaire, et l'adapter traduit
- [x] Implémentée dans `src/infrastructure/spaces/host_layout_adapter.rs`
- [x] Ce BC n'importe ni `auth::routes` ni `crate::web` — l'axe 9 passe
- [x] Écoute de `accountCreated`, POST de l'appartenance avec le profil choisi
- [x] Le contrat des trois chaînes commenté sur place, avec renvoi à la 384
- [x] Message d'erreur explicite en cas d'échec après création
- [x] ~~Fragment rendu avec la page~~ → **un endpoint**, voir ci-dessous
- [x] Pré-remplissage réparti par ce BC selon la présence d'un `@`
- [x] La ligne à deux propriétaires alignée sur sa base
- [x] `make lint`, `make check-arch`, `make test` passent — 1203 tests

## Ce qu'on a appris en la faisant

**Le pré-remplissage impose un endpoint.** La carte prévoyait un fragment rendu
avec la page ; impossible, puisque le pré-remplissage dépend de ce qui a été
cherché — donc d'une action postérieure au chargement. Le panneau est servi par
`…/admin/widgets/create-coach` et se recharge après chaque recherche.

**Le harnais ne voit qu'un bord du contrat à la fois.** La carte 380 vérifie que
l'en-tête est **posé** avec ses trois chaînes ; celle-ci vérifie que le panneau
les **écoute**. Les deux tests peuvent passer sans que les bords s'accordent —
seule la carte 384 ferme la boucle, et c'est écrit dans les deux fichiers.
