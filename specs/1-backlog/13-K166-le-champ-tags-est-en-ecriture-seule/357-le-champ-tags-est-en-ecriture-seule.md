# `common` — le champ `tags` est écrit par quatre chemins et lu par personne

**Priorité : basse** — aucune conséquence visible, mais une abstraction qui ment
**Trouvée par :** le raffinage de la carte 351
**État : à raffiner** — la question « compléter ou supprimer » n'est pas tranchée
**Fichiers pressentis :** `src/common/event_envelope.rs`,
`src/common/services/event_bus/event_tags.rs`,
`src/common/persistance/sql/find_by_tag.sql`, les `get_tags()` de quatre BCs,
`src/app/teams/io/repository/team_repository.rs`

## Le problème

`EventEnvelope` porte un champ `tags: serde_json::Value`. Quatre chemins
l'écrivent, **avec quatre formes différentes** :

| Écrivain | Ce qui atterrit dans la colonne |
|---|---|
| `auth`, `spaces`, `competitions` | `{"name":"User","value":"…"}` — un `EventTag` **seul**, pas un tableau |
| `teams` | `["treasury"]` — un tableau de chaînes, écrit **par le repository**, l'enveloppe posant `json!([])` |
| tous les app events | `json!([])`, sans exception |
| `perform_login` | `json!({ "user_id": … })` — une troisième forme, écrite à la main |

Et deux colonnes distinctes, dans deux tables : `event_log.tags`, alimentée par
`event_log_feeder` depuis les enveloppes, et `team_event_store.tags`, alimentée
directement par le repository de `teams`.

## Ce qui rend la carte plus simple qu'elle n'en a l'air

**Personne ne lit ces colonnes.**

- `find_by_tag()` — le seul lecteur de `event_log.tags`, avec sa requête
  `WHERE tags @> $1` — **n'a aucun appelant** dans le dépôt ;
- `team_event_store.tags` n'est lu par aucune requête de production ; seul un
  test de `team_repository` en vérifie le contenu.

Le mécanisme est donc **entièrement en écriture**. Quatre formes qui ne se
contredisent nulle part, faute de quelqu'un pour les comparer.

*Note pour la lecture de la carte 351 :* sa section « Trois faits établis »
affirme que « `tags` n'est pas mort — quatre modules de domain events le
remplissent ». C'est exact et incomplet : **rempli n'est pas lu.** La
correction que cette carte-là apportait visait une affirmation encore plus
fausse (« il n'est jamais rempli ») ; elle n'est pas allée jusqu'au bout.

## Pourquoi ça compte quand même

Le jour où quelqu'un voudra interroger l'event log par tag — « tous les
événements touchant l'équipe T » — l'opérateur de containment `@>` de PostgreSQL
se comporte différemment sur un objet et sur un tableau. La requête marchera
pour trois BCs et pas pour le quatrième, silencieusement. Une abstraction
inutilisée n'est pas neutre : elle promet quelque chose qu'elle ne tiendra pas.

## Les questions à trancher — pourquoi la carte est à raffiner

1. **Compléter ou supprimer ?** Un champ écrit par quatre chemins et lu par
   personne est soit un besoin réel qu'on n'a pas encore branché, soit une
   abstraction morte. Les deux réponses sont défendables ; ce qui ne l'est pas,
   c'est de laisser les quatre formes coexister.
2. **Si on complète : quelle forme canonique ?** Un tableau d'`EventTag` semble
   l'intention d'origine — `get_tags()` porte le pluriel — mais trois BCs en
   renvoient un seul, non enveloppé.
3. **Le contournement de `teams` est-il un bug ou un choix ?** Le repository
   dérive le tag du mouvement plutôt que de le poser événement par événement, et
   son commentaire l'assume : « ce serait recréer l'endroit qu'on peut
   oublier ». Ce raisonnement est bon. Mais il produit une forme incompatible
   avec celle des trois autres BCs, dans une autre table.
4. **`EventTagName` est une énumération fermée** (`User`, `Space`,
   `Competition`, `Team`) — que `teams` n'utilise pas, puisqu'il écrit la chaîne
   `"treasury"`, qui n'y figure pas.

## Ce que la carte ne doit pas faire

**Pas de migration de données existantes** tant que la question 1 n'est pas
tranchée. Réécrire des lignes d'event store pour uniformiser un champ que
personne ne lit serait du risque pur.

## Checklist — à compléter au raffinage

- [ ] « Compléter ou supprimer » est tranché, et le motif écrit
- [ ] Si compléter : la forme canonique est décidée, et les quatre écrivains s'y
      conforment
- [ ] Le sort de `find_by_tag()` est décidé — branché sur un besoin réel, ou
      supprimé avec son fichier SQL
- [ ] Le sort de `team_event_store.tags` est décidé, en tenant compte du motif
      écrit dans `team_repository`
- [ ] `make test` et `make check-arch` passent
