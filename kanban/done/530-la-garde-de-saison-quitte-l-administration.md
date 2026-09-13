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

## Le seul écart au « pas d'un iota », décidé et non glissé

La fonction journalisait ainsi :

```rust
tracing::error!("require_admin_access saison {season_id}: {e:?}");
```

Une fois l'encart appelant, cette ligne nommerait une fonction **absente du
chemin d'appel** — un journal qui désigne le mauvais coupable coûte cher à lire
en production, et c'est précisément ce que la section Observabilité du
`CLAUDE.md` cherche à éviter. Les quatre sœurs suivent la convention inverse :
`equipe_de_la_saison {team_id}`, `appariement_de_la_saison {…}`.

Elle devient `saison_de_la_competition {season_id}: {e:?}`. Ce n'est ni un import
ni une référence, donc c'en est bien une modification au sens de la règle 5 — d'où
cette section plutôt qu'un silence.

## La preuve que c'est un déplacement

Le corps déplacé diffère de l'original sur **exactement deux lignes**, et ce sont
les deux attendues :

```
< async fn verifier_saison_de_la_competition(
> pub async fn saison_de_la_competition(
<             tracing::error!("require_admin_access saison {season_id}: {e:?}");
>             tracing::error!("saison_de_la_competition {season_id}: {e:?}");
```

Le reste — le `BAD_REQUEST` sur un identifiant mal formé, la comparaison à
`competition_id`, le `404`, le commentaire qui l'explique — est passé au caractère
près. Le `diff` est la vérification, pas la relecture : c'est ce que la règle 5
demande et la seule façon de le montrer.

**Aucun fichier de test n'a été modifié**, et les 1 914 passent. C'est la preuve
que la carte demandait.

## Checklist

- [x] Copier-coller exact, y compris son commentaire — vérifié par `diff`
- [x] Renommée `saison_de_la_competition`, `pub`, dans `admin_scope.rs`, avec un
      en-tête qui dit d'où elle vient et pourquoi
- [x] `require_admin_access` l'importe ; `grep` confirme qu'il n'en reste aucune
      copie
- [x] Les tests existants passent **sans modification** — aucun fichier de test
      n'apparaît au `git status`
- [x] La ligne de journal adaptée, et l'écart écrit plutôt que glissé
- [x] `make lint`, `make check-arch`, `make test`
