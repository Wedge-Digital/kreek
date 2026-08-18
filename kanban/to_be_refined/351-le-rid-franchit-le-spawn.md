# `web` + `common` — Remonter d'une réaction à la requête qui l'a causée

**Priorité : basse** — c'est du confort de diagnostic, pas un manque bloquant
**Dépend de :** cartes 345 et 350 (sans spans des deux côtés, il n'y a rien à
relier)
**État : à raffiner** — une question de conception reste ouverte, voir plus bas
**Fichiers pressentis :** `src/common/event_envelope.rs`,
`src/web/middleware/request_log.rs`, les `to_enveloppe()` du `shared_kernel`,
les 19 listeners

## Le problème

Depuis la carte 345, on relie les listeners entre eux par `event_id` : un même
fait, plusieurs BCs qui y réagissent, un seul `grep`. Mais **on ne remonte pas
au-delà**. Un coach signale un symptôme, on part de son `x-request-id`, on
déroule sa requête… et la piste s'arrête net à l'émission de l'événement. Les
réactions qu'elle a provoquées vivent sous un autre identifiant, sans lien avec
elle.

La cause est technique et sans détour : les listeners s'exécutent dans des
tâches `tokio::spawn` distinctes, et **un span ne franchit pas un `spawn`** sans
qu'on lui passe explicitement son parent. Le `rid` meurt à la frontière de la
tâche.

C'est le chaînon qui manque pour répondre à « qu'est-ce que cette requête a
déclenché, et est-ce que ça s'est bien passé ? » — la question qu'on se pose
quand une équipe est créée mais que ses joueurs n'apparaissent pas.

## Le vocabulaire, pour ne pas s'égarer

Ce qu'on cherche à poser porte un nom dans la littérature event sourcing : un
**identifiant de causalité**. L'événement ne dit pas seulement *ce qui s'est
passé*, mais *ce qui l'a provoqué*. C'est une notion d'infrastructure, pas de
domaine : aucune règle métier n'en dépend, et aucun agrégat ne doit le lire.

## La question ouverte — pourquoi cette carte est à raffiner

**Où lire le `rid` au moment de construire l'enveloppe ?**

Le `rid` vit dans le span de la requête. Le construit, lui, se fabrique dans
`to_enveloppe()`, qui vit dans `shared_kernel` et qu'appellent le domaine et les
use cases. Trois voies, aucune évidente :

1. **Une variable de tâche (`tokio::task_local!`)**, posée par `request_log` et
   lue dans `to_enveloppe()`. Rien à changer dans les signatures — mais
   `shared_kernel` se met alors à lire un état ambiant posé par la couche web.
   C'est un couplage caché, exactement le genre que `check-arch` ne verra pas.
2. **Un champ explicite**, passé de commande en commande jusqu'à l'émission.
   Honnête et visible, mais il traverse le domaine, qui n'a aucune raison de
   connaître une notion de requête HTTP.
3. **Un tampon posé en couche IO** : le use case émet, et c'est un point de
   passage IO qui estampille l'enveloppe avant l'envoi. Reste à trouver ce point
   de passage — le bus interne est un `broadcast::Sender` nu, il n'y a pas de
   couche où se glisser aujourd'hui.

**Il faut trancher cette question avant d'écrire quoi que ce soit.** C'est ce
qui maintient la carte en `to_be_refined/`.

## Où le porter, une fois le `rid` obtenu

Deux emplacements possibles dans `EventEnvelope` :

- un **champ dédié** (`caused_by: Option<String>`) — typé et lisible, mais il
  touche tous les `to_enveloppe()` du `shared_kernel` ;
- le champ **`tags`**, qui existe déjà et n'est jamais rempli : tous les
  `to_enveloppe()` d'app events posent `json!([])`. Il faudrait étendre
  l'énumération fermée `EventTagName` (aujourd'hui `User`, `Space`,
  `Competition`, `Team`), donc une modification quand même — mais une seule.

Le second mérite un examen : un champ prévu pour porter des métadonnées et que
personne ne remplit est soit le bon endroit, soit une abstraction morte à
supprimer. Les deux réponses sont utiles à savoir.

## Ce que ça donnerait

Les listeners créeraient leur span avec le `rid` reçu en champ, et
`grep rid=01M0…` rendrait la requête **et** tout ce qu'elle a déclenché en
cascade, à travers les BCs et les tâches.

## Ce que la carte ne doit pas faire

**Pas de propagation vers les événements engendrés par un listener.** Un
listener qui émet à son tour rouvrirait la question — profondeur de chaîne,
boucles éventuelles. Un seul saut, de la requête à ses réactions directes ;
au-delà, on rediscute.

## Checklist — à compléter au raffinage

- [ ] La question « où lire le `rid` » est tranchée, et le motif écrit
- [ ] L'emplacement dans l'enveloppe est choisi (champ dédié ou `tags`)
- [ ] Le sort de `tags` est décidé : rempli, ou supprimé s'il reste mort
- [ ] Les listeners portent le `rid` reçu dans leur span
- [ ] `check-arch` reste muet — en particulier, aucun BC extractible n'acquiert
      de dépendance vers la couche web
- [ ] Vérifié en conditions réelles : une création d'équipe, un seul `grep rid=`
      montrant la requête puis les réactions de `teams` et `players`
