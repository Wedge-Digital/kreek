# L'agrégat `Space` se charge incomplet, et hors de sa souveraineté

**Priorité : haute** — préalable aux cartes 365 et 367, qui seraient fausses sans elle
**Dépend de :** rien
**Fichiers :** `src/app/spaces/io/repository/sql/space/find_space_by_id.sql`,
`src/app/spaces/io/repository/space_repository.rs`

## Pourquoi personne ne s'en est aperçu

`find_by_id` du BC `spaces` **n'a aucun appelant** dans tout le dépôt, hormis son
propre test d'intégration. L'agrégat `Space` est construit et jamais chargé en
production. Ses défauts n'ont jamais eu l'occasion de se voir.

Les cartes 365 et 367 en font le premier usage réel.

## Défaut n°1 — un membre sans avatar disparaît

```rust
let (Some(ref raw_id), Some(ref raw_name), Some(ref raw_icon), Some(ref raw_profile)) = (
    &row.coach_id, &row.coach_name, &row.coach_icon, &row.profile,
) else {
    continue;
};
```

`auth__users.coach_icon` est `VARCHAR(255)` **sans `NOT NULL`**. Un coach sans
icône fait donc `continue` : il est **silencieusement absent** de
`space.coaches`.

Le `let-else` est là pour écarter les lignes du `LEFT JOIN` d'un espace sans
membre — où les quatre colonnes sont `NULL` ensemble. Il traite au passage un
avatar manquant comme un membre manquant.

**Ce que ça casserait**, une fois les cartes 365 et 367 écrites :

| Règle | Symptôme |
|---|---|
| « la cible est-elle membre ? » | `PasMembre` sur un membre bien réel |
| le compte d'administrateurs | faux, et rendu au contrôleur comme s'il ne l'était pas |
| **l'invariant du dernier administrateur** | laisse passer un retrait en croyant en voir deux — l'espace perd son dernier admin |

Le troisième est le vrai danger : la garde ne tombe pas en panne, elle **répond
faux**.

## Défaut n°2 — `spaces` interroge `auth__users`

```sql
LEFT JOIN auth__users u ON u.id = us.coach_id
```

Une table du BC `auth`, requêtée par `spaces`. Deux règles y passent :

- la **souveraineté des données** — « il est formellement interdit à un BC
  d'effectuer des requêtes SQL sur des tables appartenant à un autre BC » ;
- le **statut extractible** de `spaces` — copier `src/app/spaces/` dans un autre
  projet ne marcherait pas, la table n'existant pas là-bas.

`spaces__user_cache` existe précisément pour ça, alimenté par
`user_created_listener`. C'est lui qu'il faut joindre. Ses colonnes sont les
mêmes, à ceci près que `coach_icon` y est **aussi** nullable — le défaut n°1 ne
se règle donc pas tout seul en changeant de table.

**L'axe 9 de `check-arch` ne l'a pas vu** : c'est un `grep`, et la référence est
dans un fichier `.sql`. Le `CLAUDE.md` le dit de ce verrou — « il ne voit ni les
chaînes littérales ni le SQL ». À traiter comme une limite connue, pas comme une
défaillance.

## La correction

Joindre `spaces__user_cache`, et distinguer les deux `NULL` :

- **un espace sans membre** — `us.coach_id IS NULL` : pas de coach à ajouter ;
- **un membre sans avatar** — `coach_icon IS NULL` : un coach à ajouter, avec
  une icône vide.

Le discriminant est `us.coach_id`, jamais `coach_icon`. `CoachIcon::try_new("")`
doit être vérifié : si le value object refuse la chaîne vide, il faut un
`CoachIcon` optionnel plutôt qu'une valeur inventée.

## Checklist

- [x] Le SQL joint `spaces__user_cache`, plus la table des comptes du BC voisin
- [x] `grep -r auth__users src/app/spaces/` ne rend rien
- [x] Le discriminant du `LEFT JOIN` est `us.coach_id`, pas l'icône
- [x] Sort de l'icône manquante tranché : **`Option<CoachIcon>`**, et le type
      l'imposait — `CoachIcon` est un alias de `CloudinaryImage`, dont la
      validation exige une URL Cloudinary. La chaîne vide étant refusée, il
      n'existait pas de « valeur neutre » à inventer
- [x] Une icône **illisible** rend le coach sans avatar plutôt que de le faire
      disparaître — c'est exactement le défaut qu'on corrige, et il aurait pu
      revenir par cette porte
- [x] Tests d'intégration sur une vraie `PgPool` :
  - [x] un espace **sans membre** rend un `Space` à `coaches` vide, pas `None`
  - [x] un membre **sans avatar** est **présent** — vu échouer sur l'ancien
        filtre, avec « un membre sans avatar a disparu »
  - [x] un espace à trois membres dont un sans avatar en rend trois
- [x] `make lint`, `make check-arch`, `make test` passent — 1093 tests

## Ce qu'on a appris en la faisant

**Le défaut était le cas général, pas un cas limite.** La carte annonçait « un
membre sans avatar disparaît ». Mesuré : **aucun** des 38 membres n'a d'icône,
donc `find_by_id` rendait `coaches` **vide pour les 26 espaces**. L'agrégat
n'était pas incomplet, il était systématiquement amputé de tous ses membres.

**Basculer la jointure était sans risque**, ce qui n'allait pas de soi : le
cache est complet — 864 lignes de part et d'autre, **zéro** membre absent.
Vérifié avant d'écrire une ligne.

**Un troisième défaut est apparu en route**, et il a fait échouer un test dont
la prémisse était fausse : `insert_user.sql` écrit `coach_icon` en `NULL` en dur
et ignore le champ qu'on lui passe. Le cache ne peut donc **jamais** stocker
d'avatar. En cherchant plus loin : **zéro utilisateur sur 864** en a un dans
`auth__users` non plus, et aucun écran ne permet d'en poser. La chaîne est morte
de bout en bout. Carte **385**.

Le semis du test insère donc en SQL direct, avec le motif en commentaire — le
test porte sur `find_by_id`, pas sur l'alimentation du cache, et il a le droit
de poser l'état qu'il veut lire.
