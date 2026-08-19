# `check-arch` — l'axe 2 tient moins que la règle qu'il annonce

**Priorité : moyenne**
**Dépend de :** rien
**Trouvée par :** le raffinage de la carte 351
**Fichiers :** `scripts/check-arch.sh`

## Le problème

L'axe 2 s'annonce « Pureté du domaine (`domain/` sans dépendance framework) ».
Ce qu'il vérifie :

```bash
grep -rnE "^use (axum|sqlx|tower|askama)(::| )" --include="*.rs" src/app/*/domain/
```

Quatre crates, nommément. Or `CLAUDE.md` dit au domaine deux choses de plus :

> `domain/` n'importe jamais de crate framework (axum, sqlx, tower, **…**)

> **Interdit** : toute dépendance framework […], accès aux ports, **appels
> async**, connaissance des repositories.

Le « … » et les « appels async » ne sont tenus par rien. **`tokio` n'est pas
dans la liste.**

## Comment on s'en est aperçu

La première version de la carte 351 proposait de lire un `tokio::task_local!`
depuis `to_enveloppe()` — dont six implémentations vivent dans `domain/`. Elle
notait elle-même que c'était « un couplage caché, exactement le genre que
`check-arch` ne verra pas ».

C'était exact, et pour une raison plus bête que prévu : `tokio` n'est pas dans
le `grep`. Le verrou n'aurait rien dit. **La carte a été écartée pour d'autres
raisons ; sans ça, elle serait passée.**

## Ce qu'il faut faire

**Ajouter `tokio` à la liste des crates interdites.** Il n'y en a aucune
occurrence dans `domain/` aujourd'hui : l'axe se pose sans rien corriger.

**Interdire `async fn` dans `domain/`, sauf dans les fichiers `*_port.rs`.**
Neuf fichiers de `domain/` déclarent des `async fn`, et **tous sans exception**
finissent par `_port.rs` :

```
competitions/domain/{group,season,competition,match_day}_repository_port.rs
match_report/domain/match_report_repository_port.rs
news/domain/{comment,article}_repository_port.rs
spaces/domain/space_repository_port/{space,user_cache}_repository_port.rs
```

Un port **déclare un contrat**, il n'appelle rien — c'est l'implémentation, qui
vit dans `io/`, qui est async pour de bon. `CLAUDE.md` place d'ailleurs les
ports dans le domaine par conception (`domain/ports/` dans la structure cible).
La convention de nommage est nette à 9 sur 9 ; l'exception se lit dans le nom du
fichier, sans liste à tenir.

## Ce que ça ne couvre pas

Le `grep` ne voit toujours ni les chaînes littérales, ni le SQL, ni une
indirection — conséquence assumée de la carte 242, qui a écarté le découpage en
crates cargo. Cet axe resserre ce qu'un `grep` peut voir, il ne le transforme
pas en compilateur.

L'axe ne dira rien non plus d'un `impl Future` écrit à la main, ni d'un
`block_on`. Les nommer explicitement serait possible ; ils n'existent pas dans
le dépôt et l'axe se durcira le jour où ils apparaîtront.

## Checklist

- [ ] `tokio` ajouté à la liste des crates interdites dans `domain/`
- [ ] `async fn` interdit dans `domain/` hors des fichiers `*_port.rs`
- [ ] Axe **vérifié sur un cas volontairement fautif**, avec assertion sur la
      substitution de test — un essai antérieur avait affiché `PASS` sur un cas
      qui n'en était pas un
- [ ] L'en-tête de `check-arch.sh` décrit l'axe 2 dans son nouveau périmètre
- [ ] `make check-arch` passe sans qu'aucun fichier n'ait dû être corrigé
