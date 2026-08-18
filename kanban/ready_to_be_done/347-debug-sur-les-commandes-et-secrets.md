# `app` — `Debug` sur les commandes, et trois secrets à masquer

**Priorité : haute** — la carte 348 en dépend, et la revue de secrets vaut par
elle-même
**Dépend de :** rien
**Fichiers :** les 56 structures `…Command` de `src/app/*/use_cases/`, dont
trois dans `auth`

## Le problème

**Aucune des 56 commandes ne dérive `Debug`.** Zéro sur 56 — elles n'ont aucune
dérivation du tout. Or la carte 348 journalise la commande reçue par chaque use
case, ce qui l'exige.

Ajouter `#[derive(Debug)]` partout est mécanique. Le faire sans regarder ce
qu'on rend imprimable ne l'est pas : **trois commandes portent des secrets en
clair.**

| Commande | Champs |
|---|---|
| `PerformLoginCommand` (`auth/use_cases/perform_login.rs`) | `password` |
| `RegisterCommand` (`auth/use_cases/register_new_acount.rs`) | `email`, `password`, `password_confirm` |
| `ResetPasswordCommand` (`auth/use_cases/reset_password.rs`) | `token`, `password`, `password_confirm` |

Un `#[derive(Debug)]` posé sans réfléchir sur ces trois-là mettrait **les mots
de passe des coachs dans `docker logs`**. Le `token` n'est pas moins grave : il
autorise la réinitialisation d'un mot de passe, c'est un identifiant de
connexion à durée de vie limitée. L'adresse e-mail, elle, est une donnée
personnelle qui n'a rien à faire dans un journal de diagnostic.

## Ce qu'il faut faire

**Les 53 autres** : `#[derive(Debug)]`, sans autre forme de procès. Quelques
types de champs devront peut-être le dériver aussi ; les value objects `nutype`
le font déjà pour l'essentiel.

**Les trois de `auth`** : un `Debug` **écrit à la main**, qui rend les champs
utiles au diagnostic (le nom de coach) et remplace les autres par un
`"[masqué]"`.

Écrire `Debug` plutôt qu'exempter ces commandes du journal, c'est le point
important de la carte : **le risque existe déjà et ne vient pas du journal.**
Aujourd'hui, n'importe quel `{:?}` égaré — dans un message d'erreur, un `dbg!`
de débogage oublié, une variante d'erreur qui embarque la commande — produit la
même fuite. Corriger l'implémentation la supprime partout à la fois, y compris
aux endroits qu'on n'a pas prévus.

## Ce qu'il faut vérifier au passage

Le tableau ci-dessus vient d'une recherche sur les noms de champs
(`password`, `token`, `secret`, `hash`, `email`). Elle ne prouve pas
l'exhaustivité : un secret nommé autrement lui échapperait. **Passer les 56
commandes en revue une par une**, c'est le seul travail non mécanique de cette
carte, et sa vraie raison d'être.

## Checklist

- [ ] Les 56 commandes dérivent `Debug`, à l'exception des trois ci-dessous
- [ ] `PerformLoginCommand`, `RegisterCommand`, `ResetPasswordCommand` ont un
      `Debug` écrit à la main, masquant mot de passe, jeton et e-mail
- [ ] Test unitaire : le rendu `{:?}` de ces trois commandes ne contient ni le
      mot de passe, ni le jeton, ni l'e-mail — c'est le seul garde-fou contre
      un `#[derive(Debug)]` réintroduit par distraction
- [ ] Les 56 commandes ont été relues une par une, pas seulement filtrées par
      nom de champ
- [ ] `make test` et `make check-arch` passent
