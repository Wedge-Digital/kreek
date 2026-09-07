# `admin_scope` ne vérifie rien d'administratif

**Priorité : basse — un nom, pas un défaut**
**Dépend de :** 530, qui y ajoute une cinquième fonction
**Sans épic**
**Fichiers :** `src/app/competitions/io/web/admin/admin_scope.rs` et ses importateurs

## Le constat

`admin_scope.rs` contient cinq vérifications « cette ressource appartient-elle à
ce parent » — `journee_de_la_saison`, `appariement_de_la_saison`,
`groupe_de_la_saison`, `equipe_de_la_saison`, et `saison_de_la_competition` que
la carte 530 y ajoute.

**Aucune ne vérifie quoi que ce soit d'administratif.** Le droit d'admin est le
travail de `require_admin_access`, dans un autre fichier.

Le nom passait inaperçu tant que seuls des handlers d'administration
l'importaient. La carte 531 — l'encart du coach connecté, que n'importe quel
coach voit — le fait mentir.

## Ce qui est en jeu

Rien de fonctionnel. Mais un module nommé `admin_*` qu'un handler public importe
invite deux erreurs : croire que l'importer suffit à contrôler un droit, ou
hésiter à l'importer parce qu'on n'est pas admin — et réécrire la vérification à
côté. La seconde est la plus probable, et c'est celle qui fait diverger un
garde.

## Conception

Renommer en `scope.rs` et le sortir de `admin/` — ces fonctions servent le BC,
pas son administration. Une dizaine d'imports à suivre.

Un renommage de fichier et d'imports, sans changement de comportement : les
tests existants doivent passer **sans modification**, et c'est la preuve.

## Checklist

- [ ] `git mv` du fichier, imports mis à jour
- [ ] Aucun changement de signature ni de corps
- [ ] Les tests passent sans être touchés
- [ ] `make lint`, `make check-arch`, `make test`
