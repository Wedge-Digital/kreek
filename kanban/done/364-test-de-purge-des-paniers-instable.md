# Le test de purge des paniers observe la mauvaise chose

**Priorité : moyenne** — il fait rougir la CI au hasard, sans qu'aucun code ne
soit en cause
**Fichier :** `src/app/teams/io/listeners/phase_basket_purge_listener.rs`
**Trouvée par :** la carte 339, en lançant la suite dix fois de suite

## Le symptôme

`une_entree_en_ready_to_play_purge_les_deux_paniers` échoue **environ une fois
sur dix** avec `make test`, et **zéro fois sur quinze** lancé seul. La
différence désigne une course, pas une faute de logique métier — le code du
listener est correct.

Un test qui rate au hasard finit ignoré, et le jour où un vrai échec s'y ajoute,
personne ne le distingue. C'est la même leçon que la carte 360.

## La cause, et elle n'est pas seulement la charge

Le listener purge les deux paniers en **deux suppressions successives**, non
atomiques :

```rust
for phase in [GamePhase::Recruitment, GamePhase::Dismissals] {
    if let Err(e) = baskets.delete(&team_id, &phase).await { … }
}
```

Le test, lui, attend en n'observant **que le premier** :

```rust
for _ in 0..40 {
    let reste = repo.load(&team_id, &GamePhase::Recruitment).await…;
    if !reste { break; }                    // ← sort dès que Recruitment est parti
    tokio::time::sleep(25 ms).await;
}
assert!(… Recruitment … is_none());
assert!(… Dismissals   … is_none());        // ← peut ne pas encore l'être
```

Il sort de sa boucle d'attente **entre les deux suppressions**. La seconde
assertion tombe alors, sur un panier qui allait disparaître un instant plus tard.

La charge n'est donc pas la cause mais l'amplificateur : sous parallélisme,
l'intervalle entre les deux `await` s'élargit, et la fenêtre s'ouvre assez
souvent pour être vue une fois sur dix.

## Deux fragilités qui s'y ajoutent

**Le budget d'attente est de une seconde** — 40 × 25 ms. Fixe, sur une base
partagée avec des dizaines de bases `#[sqlx::test]` concurrentes.

**Le test n'utilise pas `#[sqlx::test]`** mais se connecte à `DATABASE_URL` à la
main, avec `max_connections(2)`. Il partage donc la base des autres tests, et
deux connexions pour le test **et** le listener laissent peu de marge.

## Action

- [x] La boucle d'attente observe **ce qui est purgé en dernier**, ou les deux
      paniers — jamais le premier seul
- [x] Vérifier que le test échoue toujours si le listener cesse de purger le
      second panier : c'est ce qu'il est censé garder, et l'attente corrigée ne
      doit pas le rendre complaisant
- [x] Décider du reste : passer à `#[sqlx::test]` pour une base isolée, ou
      assumer la base partagée et documenter pourquoi
- [x] `make test` lancé **dix fois de suite** sans échec — un seul passage vert
      ne prouve rien sur une course

## Ce que la carte ne couvre pas

Rendre la purge atomique. Deux suppressions successives conviennent : le panier
est un brouillon, et un état intermédiaire d'une milliseconde n'a aucun lecteur.
C'est l'observation du test qui est fautive, pas le listener — et corriger le
code de production pour faire taire un test serait le mauvais sens.

## Ce qui a été fait

La boucle d'attente observe désormais **les deux** paniers, et son budget passe
de une à deux secondes. C'était la cause : sortir dès la disparition du premier
faisait tomber l'assertion sur le second, entre les deux `await` du listener.

Le reste est tranché en faveur de `#[sqlx::test]` — base isolée, plus de
`max_connections(2)` sur la base partagée. Et cela supprime au passage un défaut
que la carte n'avait pas vu :

```rust
let Some(pool) = test_pool().await else { return; };   // ← passait sans rien vérifier
```

Sans `DATABASE_URL`, le test **rendait vert sans rien exercer**. C'est la même
famille que les assertions creuses rencontrées ailleurs dans la session : un
vert qui ne prouve rien est pire qu'un rouge.

## Vérifié dans les deux sens

En faisant cesser la purge du second panier, le test tombe toujours, et sur son
message d'origine — « les deux paniers partent, pas seulement celui de la phase
quittée ». L'attente corrigée ne l'a pas rendu complaisant.

Et `make test` lancé **dix fois de suite** : 1098 passés, zéro échec. Un seul
passage vert n'aurait rien prouvé sur une course.
