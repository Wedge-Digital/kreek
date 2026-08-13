# Axe `check-arch` — le cloisonnement des espaces ne doit pas se reperdre

**Priorité : moyenne**
**À faire après :** la première migration (`318`), jamais avant
**Contexte :** `scripts/check-arch.sh`

## Ce qu'il doit attraper

Un fichier de `src/app/<bc>/io/web/` qui charge une ressource par identifiant
sans passer par le `space_scope` de son BC.

C'est le geste qu'on réintroduit sans y penser en ajoutant un handler : on
copie le voisin, on remplace `find_by_id`, et le contrôle d'appartenance
disparaît sans que rien ne proteste. Les cartes 315 à 322 corrigent
l'existant ; cet axe protège l'avenir.

## Pourquoi après la première migration, et pas avant

Tant que six BCs sont en infraction, l'axe naîtrait avec une **baseline de six
exceptions**. Le projet en a l'expérience : la baseline de l'axe 3 (cartes 300
et 301) tient encore, et une baseline de six entrées ne se résorbe jamais — elle
devient le décor.

Écrit après la 318, il naît avec cinq exceptions qui **décroissent à chaque
carte** et disparaissent avec la 322. Une baseline qui se vide toute seule est
une baseline qu'on ose regarder.

## Ce que l'axe ne pourra pas voir

C'est un `grep`, comme les autres axes — le découpage en crates ayant été
écarté (carte 242), il n'y a pas de compilateur pour l'aider.

Il ne verra donc pas :

- un chargement **indirect**, via un service ou un use case appelé depuis le
  handler ;
- un identifiant qui transite par une structure intermédiaire ;
- une ressource chargée par autre chose qu'un identifiant — une requête par
  nom, par exemple.

**Le dire dans l'en-tête de l'axe**, comme l'axe 9 le fait pour les BCs
extractibles. Un verrou dont on croit qu'il voit tout est pire qu'un verrou dont
on connaît les angles morts.

## Question ouverte : la forme de la détection

Deux pistes, à trancher au moment de l'écrire :

- **Interdire `find_by_id` dans `io/web/`** hors `space_scope.rs`. Simple, mais
  frappe large : certains chargements ne portent aucun enjeu d'espace.
- **Exiger que tout handler dont la route porte `{space_id}` et un identifiant
  appelle un `charger_*_de_l_espace`.** Plus juste, mais lier une route à son
  handler en `grep` est fragile.

La première est probablement la bonne : grossière, mais lisible, et une
exception explicite vaut mieux qu'une détection subtile qu'on ne saura pas
déboguer.

## Ce que cet axe ne remplace pas

Les scénarios e2e de chaque carte de migration. L'axe dit « le garde est
appelé » ; seul un test dit « le garde refuse ».
