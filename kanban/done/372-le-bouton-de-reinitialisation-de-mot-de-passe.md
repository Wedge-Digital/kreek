# Le bouton de réinitialisation de mot de passe

**Priorité : moyenne**
**Dépend de :** 369 — indépendante de tout le reste
**Conception :** `docs/specs/space-admin/membres/03-back.md`
**Fichiers :** `src/app/auth/{routes.rs, router.rs}`,
`src/app/auth/io/web/reset_password_request_controller.rs`,
`src/app/spaces/io/web/host_layout.rs`,
`src/infrastructure/spaces/host_layout_adapter.rs`

## Objectif

Le bouton « 🔑 Réinit. mdp » de chaque ligne envoie au coach l'email de
réinitialisation.

## Il n'y a pas de question d'autorisation, et c'est le cœur de la carte

`app::auth::router::router()` est fusionné dans `auth_app` **hors** du routeur
`protected` qui porte `require_auth` : `/auth/forgot-password` est **public**.
N'importe qui peut déjà demander un email de réinitialisation pour n'importe
quel pseudo — l'email part chez le titulaire légitime, ce qui rend l'opération
inoffensive.

Le bouton d'un administrateur **n'ajoute donc aucun privilège**. Ce n'est pas
une opération d'administration qui se trouverait vivre dans `auth` ; c'est
l'opération publique existante, avec un bouton commode.

Sans ce constat, la carte aurait demandé un port, un adapter et une garde — pour
protéger quelque chose qui n'a rien à protéger.

## La destination est injectée par l'hôte

```rust
// dans ISpacesHostLayout, aux côtés de unauthenticated_redirect()
fn password_reset_action(&self, coach_name: &str) -> String;
```

`spaces` est extractible : il ne connaît pas `auth`, il reçoit une destination
en `String` et rend son propre bouton avec ses propres classes.

**L'URL et non le markup**, contrairement au précédent `upload_widget()` du
même trait. Celui-ci injecte un fragment parce qu'un widget Cloudinary est une
mécanique qui s'appartient ; un bouton de réinitialisation est un `action-btn`
du dessin de la ligne. Le faire rendre par `auth` l'obligerait à connaître les
classes CSS de `spaces` — on déplacerait le couplage au lieu de le supprimer.

## Une route à ajouter côté `auth`

L'endpoint public actuel rend la page « consultez vos emails », ce qui n'a pas
de sens dans une ligne de tableau. Il faut une variante qui réponde
`HX-Trigger: showToast` sans swap.

C'est une route d'`auth`, décidée et écrite par `auth`. `spaces` n'en connaît
que l'adresse, et par injection.

**Aucun use case côté `spaces`.** Le BC ne fait que rendre un bouton.

## Checklist

- [x] Route `auth` rendant **204** sans contenu ni redirection, réutilisant
      `send_reset_password_email`
- [ ] ~~`HX-Trigger: showToast`~~ — **écarté** : aucun mécanisme de toast
      n'existe dans le projet. En construire un serait un dispositif d'interface
      transverse, utile mais qui n'a pas à se décider au détour de cette carte.
      Le bouton gère son propre retour, en local
- [x] `password_reset_action()` ajoutée à `ISpacesHostLayout`
- [x] Implémentée dans `src/infrastructure/spaces/host_layout_adapter.rs`
- [x] ~~`MemberRowVm.reset_action`~~ → **portée par le gabarit** : l'URL est la
      même pour toutes les lignes, la mettre dans le VM l'aurait recopiée à
      chaque membre
- [x] `spaces` n'importe ni `auth::routes`, ni `crate::web` — vérifié, et l'axe 9
      passe
- [x] Le comportement est le même pour tout le monde, **soi-même compris**
- [x] `make lint`, `make check-arch`, `make test` passent — 1140 tests

## Ce qu'on a appris en la faisant

**Une seconde raison de ne pas réutiliser l'endpoint public**, que la carte
n'avait pas vue : il répond `HX-Redirect` vers la page « consultez vos emails ».
Invoqué depuis une ligne de tableau, il ferait **quitter l'application**. La
carte n'invoquait que le rendu d'une page au lieu d'un fragment.

**Le retour visuel n'est pas optimiste.** Le libellé ne bascule en « Envoyé ✓ »
que si `event.detail.successful` est vrai. Un basculement au clic mentirait à la
première panne d'envoi.

**Un pseudo inconnu rend 204 comme un pseudo connu**, avec son test. Distinguer
les deux dirait à n'importe qui si un compte existe — c'est le choix déjà fait
par l'endpoint public, repris plutôt que réinventé.

**La doublure du trait porte une URL volontairement quelconque.** Ajouter une
méthode à `ISpacesHostLayout` a cassé `FakeHostLayout`. Lui donner la vraie
destination aurait laissé croire que le BC la connaît ; `/destination-de-l-hote`
prouve le contraire.
