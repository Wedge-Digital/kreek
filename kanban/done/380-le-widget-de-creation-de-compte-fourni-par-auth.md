# Le widget de création de compte, fourni par `auth`

**Priorité : haute**
**Dépend de :** 379
**Conception :** `docs/specs/space-admin/ajout-direct/{03-back.md, 07-integration.md}`
**Fichiers :** `src/app/auth/{routes.rs, router.rs}`,
`io/web/coach_creation_widget.rs`, `io/web/templates/widgets/coach-creation.html`,
la feuille d'`auth` correspondante

## Objectif

Un fragment autonome : Pseudo, Email, bouton. Il valide, affiche **ses** erreurs
chez lui, et en cas de succès pose

```
HX-Trigger: {"accountCreated": {"coach_id": "01J…", "name": "NurgleFan"}}
```

## Pourquoi une carte à part

Elle n'a aucune valeur tant que `spaces` ne l'affiche pas — c'est la carte 383
qui l'affiche. Mais elle vit **entièrement dans `auth`** : route, contrôleur,
gabarit, feuille, tests.

Un BC dont on maintient l'indépendance ne se livre pas dans le même commit que
son consommateur. Le jour où quelqu'un se demandera ce qu'`auth` expose, la
réponse doit tenir dans son propre historique.

## Le contrat que rien ne vérifie

Le nom `accountCreated` et les clés `coach_id` et `name` franchissent la
frontière entre deux BCs **par le navigateur**. Ni le compilateur, ni
`cargo test`, ni `check-arch` — un `grep` aveugle aux chaînes littérales et aux
attributs HTML — ne les voient.

**Si une clé est renommée ici, seule la carte 384 le dira.** À commenter sur
place, des deux côtés.

## La route est publique, et ce n'est pas un oubli

Tout le routeur d'`auth` l'est : il est fusionné dans `auth_app` **hors** du
routeur `protected` qui porte `require_auth`, et `/auth/register` crée déjà des
comptes sans authentification.

Cette route ne fait rien de plus : elle rend un fragment au lieu d'une page, et
sans mot de passe. **La garde qui compte est côté `spaces`**, sur l'ajout —
créer un compte n'ajoute personne à un espace.

## Le style vient d'`auth`, jamais de `spaces`

`auth` **sert ses propres feuilles** — l'exception documentée de la règle du
bundle, ses pages étant des chargements complets sans swap. Son widget suit la
même règle et **n'entre pas dans `FEUILLES_APP`**.

Il s'accorde au reste par les **tokens de `common.css`** (`--p1`, `--text-tiny`,
`--radius-*`), qui sont globaux. **Aucune classe de `spaces`** : c'est ce qui
garde le couplage à zéro, et c'est la raison pour laquelle on a choisi le widget
plutôt qu'un port.

## Le pré-remplissage

`CoachPrefill { pseudo: Option<…>, email: Option<…> }`, reçu en query. Deux
champs ciblés plutôt qu'une chaîne à répartir : **la répartition est une décision
de `spaces`**, qui sait ce que l'utilisateur cherchait. Faire trancher `auth` sur
la présence d'un `@` lui ferait deviner une intention qu'il n'observe pas.

## Checklist

- [x] Route `COACH_CREATION_WIDGET`, GET et POST
- [x] Gabarit : Pseudo, Email, bouton — **pas de sélecteur de profil**, testé
- [x] Erreurs rendues **dans le fragment**, jamais remontées à l'appelant, et
      **aucun `HX-Trigger` posé sur un échec** — l'appelant ne peut pas confondre
      un refus avec un succès silencieux
- [x] `HX-Trigger: accountCreated` avec `coach_id` et `name`, commenté comme
      contrat non vérifié, et **vu échouer** en renommant les clés
- [x] Pré-remplissage par deux champs ciblés, décidés par l'appelant
- [ ] ~~Feuille servie par `auth`, absente de `FEUILLES_APP`~~ — **inversé**,
      voir ci-dessous. Elle est au bundle, et n'utilise que des tokens globaux
- [x] `auth` n'importe rien de l'hôte — l'axe 9 passe
- [x] `make lint`, `make check-arch`, `make test` passent — 1190 tests

## Ce qu'on a appris en la faisant

**La carte se trompait sur la feuille.** Elle prévoyait que ce BC serve la sienne,
comme il le fait déjà, et qu'elle reste hors du bundle. Mais l'exclusion vise
**ses pages** — des chargements complets sans swap, qui chargent leurs feuilles
par `<link>` et bloquent le rendu comme elles le doivent.

Ce fragment-ci est affiché dans une page de l'hôte, déjà stylée par le bundle. Un
`<link>` inséré au swap ne bloquerait pas le rendu et produirait exactement le
clignotement supprimé par la carte 342. La feuille entre donc au bundle, et
l'axe 9 reste vert : c'est **l'hôte** qui la liste, ce BC ne référence rien de
lui.

**Le harnais verrouille ce qu'il peut du contrat non typé** — que l'en-tête est
posé avec les bonnes clés. Il ne peut pas vérifier que quelqu'un les écoute :
cela reste pour la carte 384.
