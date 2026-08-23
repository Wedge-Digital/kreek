# L'avatar d'un coach n'existe nulle part

**Priorité : moyenne** — rien n'est cassé, mais trois colonnes et un champ de
value object entretiennent l'illusion d'une fonctionnalité qui n'a jamais existé
**Dépend de :** rien
**Trouvée par :** la carte 375, dont un test a échoué sur une prémisse fausse

## Le constat

La colonne existe dans trois tables — `auth__users.coach_icon`,
`spaces__user_cache.coach_icon`, et le champ `icon` de plusieurs types domaine.
Mesuré sur la base de démonstration :

| Mesure | Valeur |
|---|---|
| Utilisateurs dans `auth__users` | 864 |
| **Utilisateurs ayant un avatar** | **0** |
| Membres d'espaces | 38 |
| **Membres ayant un avatar** | **0** |

Et la chaîne est coupée en trois endroits indépendants :

**Aucun écran ne permet d'en poser un.** `grep -rl "coach_icon\|avatar"` sur
`src/app/auth/io/web/` et `src/web/` ne rend rien.

**L'app event ne le transporte pas.** `AuthAppEvent::AccountCreated` porte
`user_id`, `user_name` et `email`. Rien d'autre, et il n'existe aucun événement
signalant un changement d'avatar.

**Le cache l'écrase.** `spaces/io/repository/sql/user_cache/insert_user.sql`
écrit la colonne en `NULL` **en dur** :

```sql
INSERT INTO spaces__user_cache (id, coach_name, coach_icon, email)
VALUES ($1, $2, NULL, $3)
```

`add_user` reçoit pourtant un `User` qui porte `icon: Option<CloudinaryImage>`,
et le jette en silence. Ce n'est probablement pas un oubli : à la création d'un
compte il n'y a pas d'avatar à écrire, et le `NULL` en dur est cohérent avec ce
que l'événement transporte. Mais rien ne le dit, et la signature promet le
contraire.

## Ce que ça a coûté

La carte 375 corrigeait un chargement qui faisait disparaître de l'agrégat tout
membre sans avatar. Son test « un membre **avec** avatar est chargé avec son
avatar » a échoué : le semis passait par `add_user`, qui écrasait la valeur.

Le test a dû insérer en SQL direct, avec le motif en commentaire. C'est
acceptable pour un test du chargement, mais c'est une prémisse fausse trouvée
par accident — pas par une vérification.

## Ce qui est à trancher

C'est une décision produit avant d'être une décision technique, d'où le
raffinement.

**Implémenter.** Un écran de compte pour téléverser un avatar — le widget
Cloudinary existe déjà et `ISpacesHostLayout::upload_widget()` sait l'injecter —
plus un app event qui porte l'icône à la création et un second qui signale son
changement, plus l'alimentation du cache. C'est la seule voie qui rende la
colonne honnête.

**Supprimer.** Retirer la colonne des trois tables, le champ des types domaine,
et rendre partout des initiales — ce que fait déjà `crate::common::initials`,
utilisé par `coach_search_results`. C'est ce que l'application affiche
aujourd'hui, sans l'avoir décidé.

**Assumer par écrit.** Garder les colonnes pour plus tard, et documenter que
`coach_icon` est structurellement `NULL`. C'est la voie qui laisse le piège en
place ; à ne retenir que si l'implémentation est déjà planifiée.

## Ce que la carte ne couvre pas

**Le logo d'un espace**, qui lui fonctionne : `spaces.space_icon_path` est
alimenté, et `CloudinaryImage` y porte de vraies URLs. Ne pas confondre les
deux — c'est le coach qui n'a pas d'image, pas l'espace.

## Questions à trancher au raffinement

- Un avatar de coach est-il attendu par les utilisateurs, ou les initiales
  suffisent-elles ? La maquette d'administration affiche des initiales
  colorées, et personne ne s'en est plaint.
- Si on implémente : l'avatar appartient-il à `auth` (c'est un attribut de
  compte) ou à chaque espace (un coach pourrait s'y présenter autrement) ?
- Si on supprime : que faire des colonnes en base — migration de suppression, ou
  abandon en place avec un commentaire ?
