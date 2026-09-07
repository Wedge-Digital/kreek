# La garde de saison quitte l'administration

**Priorité : moyenne — préalable à l'encart, utile en soi**
**Épic :** E16 — Sondage de présence
**Dépend de :** rien
**Fichiers :** `src/app/competitions/io/web/admin/admin_page.rs`,
`src/app/competitions/io/web/admin/admin_scope.rs`

## Le constat

`require_admin_access` fait **deux** choses : vérifier le droit
d'administration, et vérifier que la saison appartient à la compétition. La
seconde est `verifier_saison_de_la_competition`, **privée** dans
`admin_page.rs`.

L'encart du coach connecté (carte 531) a besoin de la seconde sans la première :
n'importe quel coach le voit, et un `season_id` d'une autre compétition doit
rendre `404`.

## Conception

Elle déménage dans `admin_scope.rs`, auprès de ses quatre sœurs —
`journee_de_la_saison`, `appariement_de_la_saison`, `groupe_de_la_saison`,
`equipe_de_la_saison` — qui sont toutes des vérifications « cette ressource
appartient-elle à ce parent ». Elle y prend le nom de la famille :
`saison_de_la_competition`.

`require_admin_access` l'importe au lieu de la contenir. **Son comportement ne
change pas d'un iota** : c'est un déplacement, pas une refonte.

## Le point de vigilance

**Copier-coller exact, imports adaptés, jamais réécrite** — règle 5 du
`CLAUDE.md`. Son `404` volontaire porte un motif écrit :

> `404` et non `403` : une saison qui n'appartient pas à cette compétition est
> hors du périmètre du chemin. Répondre `403` confirmerait son existence à qui
> se contente d'essayer des identifiants.

Ce commentaire part avec la fonction. Une réécriture « propre » le perdrait, et
le prochain lecteur corrigerait le `404` en `403` en croyant bien faire.

## Ce que cette carte ne fait pas

**Elle ne renomme pas `admin_scope`.** Le nom devient discutable dès qu'un
handler non-admin l'importe — ces fonctions n'ont d'ailleurs jamais rien vérifié
d'administratif. C'est un défaut de nom, pas de conception, et le corriger touche
une dizaine d'imports : c'est l'objet de la carte 534, hors épic.

## Checklist

- [ ] `git mv` de la fonction — copier-coller exact, y compris son commentaire
- [ ] Renommée `saison_de_la_competition`, `pub`, dans `admin_scope.rs`
- [ ] `require_admin_access` l'importe ; vérifier qu'il n'en reste aucune copie
- [ ] Les tests existants de l'administration passent **sans modification** —
      c'est la preuve que le déplacement n'a rien changé
- [ ] `make lint`, `make check-arch`, `make test`
