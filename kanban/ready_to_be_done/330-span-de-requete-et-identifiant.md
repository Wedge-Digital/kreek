# `web` — Un identifiant sur chaque ligne, et de quoi le retrouver

**Priorité : haute** — c'est ce qui transforme des cris isolés en récits
**Dépend de :** carte 329 (le journal doit exister avant qu'on l'enrichisse)
**Fichiers :** `src/main.rs`, `src/web/middleware/request_log.rs`, les cinq
listeners d'app events qui déclenchent un use case

## Le problème

Les 198 `tracing::error!` du code sont des lignes **isolées**. Rien ne dit
laquelle appartient à quelle requête, ni quel coach l'a provoquée, ni ce qui
s'était passé juste avant. En production, un incident se présente donc comme
une ligne unique, hors contexte.

C'est exactement ce que les **spans** de `tracing` résolvent : ouvrir un span
par requête fait que *toute* ligne émise en dessous en hérite
automatiquement — sans passer quoi que ce soit en paramètre, et **sans toucher
aux 198 appels existants**. C'est le meilleur rapport valeur/effort de toute
l'épic.

Reste la question pratique, celle qui décide de l'utilité réelle du dispositif :
**comment obtient-on l'identifiant à grepper** quand un coach signale un
problème ? Sans réponse à ça, on obtient des identifiants qui corrèlent bien
les lignes entre elles mais dont on ne connaît jamais la valeur.

## Ce qu'il faut faire

**Un span par point d'entrée.** Il y en a deux, et le second compte autant que
le premier.

**1. La requête HTTP.** `request_log` ouvre un span portant :

| Champ | Pourquoi |
|---|---|
| `rid` | l'identifiant de corrélation — un ULID, le projet a déjà `SUlid` |
| `method`, `path` | ce qui était demandé |
| `coach` | qui demandait, quand la session est authentifiée |
| `space` | l'espace concerné, quand il est dans le chemin |

**2. L'app event.** Les cinq listeners qui déclenchent un use case s'exécutent
**hors de toute requête** : pas d'utilisateur, pas de chemin, rien pour les
rattacher à quoi que ce soit. Aujourd'hui, quand une projection ne se met pas à
jour, il n'existe aucune trace du passage. Chaque listener ouvre donc son propre
span, portant l'événement traité et son identifiant.

**3. L'écho en en-tête.** La réponse porte `x-request-id` avec le `rid`. C'est
lui qui fait le lien entre le symptôme et le journal : le coach ou le
développeur lit l'identifiant dans l'onglet réseau du navigateur, et
`docker logs … | grep rid=<valeur>` rend la requête entière.

**4. Les durées.** Elles ne viennent pas du span mais du souscripteur, en une
ligne dans `main.rs` : `with_span_events(FmtSpan::CLOSE)` fait émettre une
ligne à la fermeture de chaque span, temps passé compris. Réglage global, qui
servira aussi aux spans de use cases de la carte 333.

## Le point de vigilance : les champs doivent s'imprimer

Un span n'a d'intérêt ici que si ses champs apparaissent **physiquement sur
chaque ligne** — c'est ce qui rend `grep` opérant, et c'est toute la valeur du
dispositif. Le formateur par défaut de `tracing_subscriber::fmt` imprime bien le
contexte de span, mais **c'est à vérifier explicitement sur une sortie réelle**,
pas à supposer.

Deux corollaires, imposés par l'usage en terminal :

- **une ligne par événement**, jamais plus : `grep` est orienté ligne. Ça
  condamne le format `pretty()`, et ça demande de se méfier des `{e:?}` de
  certaines erreurs `sqlx`, qui produisent des retours à la ligne.
- **pas de JSON.** Le besoin est `docker logs` et `grep` ; le texte lisible
  reste le bon choix.

## Checklist

- [ ] Span de requête avec `rid`, `method`, `path`, et `coach` / `space` quand
      ils sont connus
- [ ] Span par listener d'app event, portant l'événement et son identifiant
- [ ] En-tête `x-request-id` sur la réponse, valeur identique au `rid` du span
- [ ] `with_span_events(FmtSpan::CLOSE)` — les durées apparaissent
- [ ] **Vérifié sur une sortie réelle** que le `rid` figure sur chaque ligne
      émise pendant la requête, y compris les `error!` existants non modifiés
- [ ] Vérifié qu'aucun événement ne s'étale sur plusieurs lignes
- [ ] Test e2e : une réponse porte `x-request-id`
- [ ] `make test` et `make check-arch` passent
