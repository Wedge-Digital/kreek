# `config` — Le niveau de journalisation rejoint la configuration

**Priorité : moyenne**
**Dépend de :** rien (mais n'a d'intérêt qu'avec un journal qui existe — cf. 344)
**Fichiers :** `src/main.rs`, `src/config.rs`

## Le problème

Le filtre est écrit en dur, avec un seul repli :

```rust
tracing_subscriber::EnvFilter::try_from_default_env()
    .unwrap_or_else(|_| "kreek=debug".into())
```

Deux défauts, opposés l'un à l'autre.

**Trop.** `kreek=debug` en production, c'est tous les `debug!` du projet — et
tous ceux qu'on ajoutera — expédiés dans `docker logs`. Le niveau de défaut
d'une application déployée est `info`.

**Trop peu.** Une directive ciblée n'active *que* sa cible : rien de `sqlx`,
rien de `tower_http`, rien d'aucune dépendance ne passe. On est **aveugle sur
la couche SQL**, y compris sur les erreurs de connexion et les requêtes lentes.

Et le réglage n'est atteignable que par `RUST_LOG`, alors que **tout le reste
de la configuration passe par `AppConfig`** au format `APP__<SECTION>__<CLÉ>`.
Changer le niveau en production suppose donc de connaître une variable qui ne
ressemble à aucune autre.

## Ce qu'il faut faire

Un `APP__LOG__LEVEL` dans `AppConfig`, valeur par défaut `info`, qui construit
le filtre au démarrage.

**`RUST_LOG` garde la priorité quand il est posé.** C'est l'échappatoire
d'investigation : on ouvre `kreek::app::players=debug` le temps d'un incident,
sans toucher à la configuration déployée. La règle est simple — `RUST_LOG` s'il
existe, sinon le niveau de `AppConfig`.

Le filtre construit doit ouvrir `sqlx` au moins en `warn`, pour que les échecs
de requête cessent d'être invisibles.

## Ce qu'on ne fait pas

Pas de rechargement à chaud du niveau. Ça demanderait un `reload::Handle` et un
point d'entrée pour le piloter — beaucoup de mécanique pour un besoin qu'un
redémarrage de conteneur couvre.

## À vérifier en même temps — la rotation Docker

L'application écrit sur la sortie standard, et le démon Docker la capture : rien
à changer côté application. Mais le pilote `json-file` **ne tourne pas par
défaut** — le fichier grossit sans fin, et il disparaît à la recréation du
conteneur.

Vérifier que `max-size` et `max-file` sont posés, et le noter dans la carte.
Ce n'est pas du code, mais c'est le préalable sans lequel toute l'épic ne sert
à rien : le meilleur journal du monde ne vaut rien s'il s'évapore.

## Checklist

- [ ] `APP__LOG__LEVEL` dans `AppConfig`, défaut `info`
- [ ] `RUST_LOG` prime quand il est posé
- [ ] `sqlx` ouvert au moins en `warn`
- [ ] Le fichier `.env` d'exemple et la documentation de configuration
      mentionnent la nouvelle variable
- [ ] Rotation du pilote de logs Docker vérifiée (`max-size`, `max-file`)
- [ ] `make test` et `make check-arch` passent
