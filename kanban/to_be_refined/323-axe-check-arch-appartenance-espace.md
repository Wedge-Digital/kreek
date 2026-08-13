# Axe `check-arch` — le cloisonnement des espaces ne doit pas se reperdre

**Priorité : moyenne**
**À faire après :** la carte `324` et la première migration (`318`)
**Contexte :** `scripts/check-arch.sh`

## Ce qu'il doit attraper

**Depuis la carte 324, la cible a changé — et l'axe devient bien plus simple.**

Le contrôle n'est plus un appel dans chaque handler mais un middleware commun
alimenté par un résolveur `ISpaceOwnership` par BC. L'axe n'a donc plus à
inspecter des chargements de ressource : il vérifie qu'un BC exposant une route
`/app/{space_id}/…/{ressource_id}` a bien **un résolveur enregistré** dans
`main.rs`.

C'est une comparaison entre deux listes — les BCs dont `routes.rs` porte un
identifiant de ressource, et ceux dont un résolveur est enregistré — au lieu
d'une traque de `find_by_id` dans quinze fichiers.

Le geste qu'on veut empêcher n'est plus « oublier un appel » mais « exposer un
nouveau BC sans son résolveur ». Il est plus rare, et beaucoup plus visible.

## Pourquoi après la première migration, et pas avant

Tant que six BCs sont en infraction, l'axe naîtrait avec une **baseline de six
exceptions**. Le projet en a l'expérience : la baseline de l'axe 3 (cartes 300
et 301) tient encore, et une baseline de six entrées ne se résorbe jamais — elle
devient le décor.

Écrit après la 318, il naît avec cinq exceptions qui **décroissent à chaque
carte** et disparaissent avec la 322. Une baseline qui se vide toute seule est
une baseline qu'on ose regarder.

Depuis la carte 324, les exceptions sont des BCs sans résolveur — une ligne
chacune, et leur disparition est mécanique.

## Ce que l'axe ne pourra pas voir

C'est un `grep`, comme les autres axes — le découpage en crates ayant été
écarté (carte 242), il n'y a pas de compilateur pour l'aider.

Il dira qu'un résolveur **existe**, jamais qu'il est **juste**. Un résolveur qui
rendrait `None` sur toutes les ressources laisserait passer tout le trafic, et
l'axe le trouverait conforme. Seuls les tests de handler de chaque carte
attrapent ça.

### L'angle mort du middleware lui-même, à connaître

Le middleware ne lit que les **paramètres de chemin**. Une ressource désignée
autrement — par une chaîne de requête, par un champ de formulaire — échappe au
contrôle sans que rien ne le signale.

Le projet en a au moins un cas : le panneau de customisation reçoit
`line_id` et `skill_id` dans le corps de ses `POST`. Ils sont sans enjeu ici,
puisque le `player_id` du chemin est, lui, contrôlé — mais la règle générale
mérite d'être écrite : **si une route désigne une ressource hors du chemin, le
middleware ne la voit pas.**

**Le dire dans l'en-tête de l'axe**, comme l'axe 9 le fait pour les BCs
extractibles. Un verrou dont on croit qu'il voit tout est pire qu'un verrou dont
on connaît les angles morts.

## La question ouverte est close par la carte 324

Elle portait sur la forme de la détection — interdire `find_by_id` dans
`io/web/`, ou lier chaque route à son handler. Les deux étaient fragiles.

Le middleware commun la rend sans objet : il n'y a plus qu'une liste à
comparer à une autre.

## Ce que cet axe ne remplace pas

Les scénarios e2e de chaque carte de migration. L'axe dit « le garde est
appelé » ; seul un test dit « le garde refuse ».
