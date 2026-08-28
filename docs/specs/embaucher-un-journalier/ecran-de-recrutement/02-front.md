# Écran de recrutement · Phase 2 : architecture front

**Conception** : `../00-conception.md` · **Maquette** :
`assets/rawpages/html/app-team-recruitment.html`

## L'assemblage existe déjà, et il ne bouge pas

`teams-recruitment.html` est **déjà** une page d'assemblage à deux widgets :

```html
<div class="rec-main" hx-get="{{ catalog_url }}"
     hx-trigger="load, basketChanged from:body" hx-target="this" hx-swap="innerHTML"></div>

<div class="rec-side" hx-get="{{ cart_url }}"
     hx-trigger="load, basketChanged from:body" hx-target="this" hx-swap="innerHTML"></div>
```

| Widget | BC | Endpoint | Trigger |
|---|---|---|---|
| `.rec-main` — le catalogue | `teams` | `recruitment_catalog_widget` | `load`, `basketChanged` |
| `.rec-side` — le panier | `teams` | `recruitment_cart_widget` | `load`, `basketChanged` |

**Cette fonctionnalité n'ajoute aucun widget.** La section des journaliers entre
dans le **catalogue**, qui se recharge déjà sur `basketChanged` — donc recruter
un journalier le retire de la liste sans un mécanisme de plus.

### Pourquoi pas un troisième widget

Il serait tentant d'en faire une section autonome, chargée par son propre
`hx-get`. Trois raisons de ne pas le faire :

- **Le budget est commun.** Recruter un journalier change ce qu'on peut encore
  acheter au catalogue ; deux widgets séparés devraient se prévenir l'un
  l'autre, alors qu'un seul se recharge d'un bloc.
- **La limite de 16 est commune** (décision 11) : un journalier recruté compte
  parmi les permanents, et le catalogue doit le savoir immédiatement.
- **`basketChanged` fait déjà le travail.** Ajouter un widget, c'est ajouter un
  abonnement au même événement pour afficher une donnée que le premier a déjà
  chargée.

## Ce que le catalogue gagne

Un panneau, **au-dessus** de « Recruter un joueur ».

```
┌─ Journaliers du dernier match ────────────── (filet ambré)
│  ⏳ Ils partent à la fin de cette phase.
│  ┌──────────────┬────────────┬──────────────┬───────┬──────────┐
│  │ Journalier   │ Expérience │ Amélioration │ Prix  │          │
│  │ Gwenn ar…    │ 6 PSP      │ Blocage      │ 85    │ Recruter │
│  │ Trois-quart  │            │              │ 65+20 │          │
└──────────────────────────────────────────────────────────────┘
┌─ Recruter un joueur ─────────────────────────────────────────┐
```

**Le panneau disparaît quand la liste est vide.** La plupart des matchs se
jouent sans journalier, et un panneau vide poserait la question « qu'est-ce que
j'ai raté ? » à chaque phase de recrutement.

### Les quatre colonnes, et ce qu'elles disent

| Colonne | Contenu | Pourquoi |
|---|---|---|
| Journalier | le nom, et le poste en dessous | c'est un joueur nommé, pas un poste |
| Expérience | « 6 PSP », grisé à zéro | **l'argument de la décision** — c'est pour eux qu'on paie plus cher |
| Amélioration | la compétence prise, ou « aucune » | ce qui justifie l'écart de prix |
| Prix | la valeur courante, **décomposée** si elle dépasse le tarif du poste | « 65 + 20 d'amélioration » — sans quoi le coach croit à une erreur |

La décomposition n'est pas un ornement : c'est la formule du LRB — « frais
d'embauche plus hausse de valeur » — rendue lisible à l'endroit où elle
s'applique, plutôt qu'expliquée dans une aide.

### L'urgence est portée par la forme

Filet ambré en haut du panneau, position au-dessus du catalogue, et un
avertissement en clair. Un poste du catalogue sera encore là au prochain match ;
un journalier non recruté est **perdu**, avec son expérience.

C'est la seule différence de nature entre les deux panneaux, et elle doit se
voir sans être lue.

## Le panier

Un journalier recruté entre dans le panier comme les autres lignes, avec la
mention de son statut :

```
Gwenn ar Skorn — journalier          − 85 kPo   ×
```

**Il est retirable comme les autres** (décision confirmée en phase 1). Le retirer
signifie renoncer définitivement — mais c'est aussi vrai de ne jamais l'avoir
ajouté, et une confirmation ici serait une exception dans un panier qui n'en a
aucune.

## Ce qui reste front

**Rien de neuf.** Le catalogue est rendu par le serveur, le panier aussi, et
`basketChanged` les resynchronise. La maquette porte du JS parce qu'elle est
statique ; l'écran réel n'en a pas besoin.

## Ce que la page ne fait pas

- **Aucun accès au détail du journalier.** On voit son nom, ses SPP, son
  amélioration — assez pour décider. Sa fiche complète est ailleurs.
- **Aucun tri, aucun filtre** : il y en a au plus quelques-uns.
- **Aucune confirmation** au retrait du panier.

## Règles métier

**Aucune à préciser.** Les quinze décisions de `00-conception.md` couvrent la
fonctionnalité, et cette phase n'en fait apparaître aucune — l'écran existait,
il gagne un panneau.

Deux points restent à trancher en phase 3, et ils sont techniques :

1. **Où la liste des journaliers est-elle lue ?** `ISquadPort` rend déjà
   l'effectif avec `value_kpo` et `spp` ; il lui manque de dire lesquels sont
   provisoires (décision 6, `is_temporary`).
2. **Comment l'amélioration prise est-elle affichée ?** Le DTO d'effectif ne la
   porte pas. Il faudra soit l'y ajouter, soit la déduire de l'écart entre la
   valeur courante et le tarif du poste — ce second chemin donnant un nombre
   mais pas un nom.
