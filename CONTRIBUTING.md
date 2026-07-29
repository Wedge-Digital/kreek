# Contribuer à Kreek

Merci de votre intérêt pour Kreek. Ce document décrit les règles de contribution.
Les lire avant d'ouvrir une PR vous évitera — et nous évitera — du travail inutile :
les contributions qui ne respectent pas l'architecture ou le processus décrits ici
seront refusées, quelle que soit leur qualité intrinsèque.

## Avant toute contribution : le CLA

Kreek est publié sous **AGPL-3.0-or-later**, avec une licence commerciale
disponible séparément (voir `LICENSE` et le README). Pour que ce double régime
reste juridiquement possible, **toute contribution est conditionnée à la signature
d'un Contributor License Agreement (CLA)** accordant à Wedge Digital les droits
nécessaires pour distribuer votre contribution sous les deux licences.

Concrètement :

- Vous restez titulaire du droit d'auteur sur votre contribution.
- Vous accordez à Wedge Digital une licence irrévocable, mondiale et gratuite
  d'utiliser, modifier, sous-licencier et redistribuer votre contribution,
  y compris sous des termes autres que l'AGPL.
- Vous garantissez que la contribution est votre œuvre originale (ou que vous
  disposez des droits nécessaires) et qu'elle n'est soumise à aucune obligation
  incompatible (contrat de travail, licence tierce, etc.).

La signature se fait lors de votre première PR : contactez
<bertrand.begouin@wedge-digital.com> pour recevoir le document.
Aucune PR n'est mergée sans CLA signé.

**Contributions générées par IA** : elles sont acceptées dans les mêmes conditions
que les autres — vous en assumez la paternité et les garanties du CLA. Vous devez
avoir relu et compris chaque ligne que vous soumettez.

## Périmètre du projet

Kreek est un **moteur de gestion de ligue** : il ne contient aucune règle de jeu.
Les règles sont fournies au démarrage via un corpus de références
(`docs/reference-data-schema.md`). Toute PR qui introduit des données de règles
sous copyright tiers dans le dépôt sera refusée. Le jeu de démonstration
(`assets/references.example/`) est fictif et doit le rester.

## Mise en place de l'environnement

Prérequis : Rust stable récent, PostgreSQL, `sqlx-cli`.

```bash
cp .env.example .env            # renseigner vos valeurs
sqlx migrate run                # migrations
cp scripts/seed_accounts.example.json scripts/seed_accounts.json
cargo run -- seed-accounts --input scripts/seed_accounts.json
```

`BYPASS_AUTH=true` est réservé au développement local. Ne l'activez jamais dans
une configuration destinée à un environnement exposé, et ne soumettez aucune PR
qui en assouplit les garde-fous.

## Architecture : les règles non négociables

Kreek suit une architecture DDD / CQRS / event sourcing avec des bounded contexts
(BC) isolés au niveau des unités de compilation (workspace Cargo). Les règles
détaillées vivent dans `CLAUDE.md` et sont vérifiées mécaniquement.

Les invariants que toute PR doit respecter :

1. **Isolation des BC** — un BC ne dépend jamais directement d'un autre BC.
   Les types partagés passent par le crate `shared_kernel`, rien d'autre.
2. **Point d'entrée contractuel** — chaque BC expose `fn router() -> axum::Router`
   comme unique surface HTTP.
3. **Value objects** — les types métier utilisent le pattern newtype (nutype).
   Pas de `String` ou `i64` nus dans les signatures de domaine.
4. **Event sourcing** — l'état des agrégats dérive des événements. On n'ajoute
   pas de mutation d'état qui contourne l'event store.
5. **Rendu serveur** — Askama + HTMX. Pas de framework JS, pas de SPA,
   pas de build front. Alpine.js est toléré pour les micro-interactions
   existantes, sans extension de son rôle.

Le gate mécanique :

```bash
make check-arch
```

Une PR dont `make check-arch` échoue n'est pas revue.

## Tests

- Les tests d'intégration utilisent `#[sqlx::test]` (isolation par clonage de
  template PostgreSQL, exécution parallèle).
- Toute nouvelle commande, tout nouvel événement, toute nouvelle projection
  arrive avec ses tests.
- Les tests e2e s'appuient sur le compte de seed (`legacy_id: 1`).

```bash
cargo test
```

## Workflow de contribution

1. **Ouvrez une issue d'abord** pour toute modification non triviale.
   Décrivez le problème et l'approche envisagée. Attendez un accord de principe
   avant d'écrire du code : le projet a une direction architecturale forte et
   toutes les fonctionnalités pertinentes n'ont pas vocation à y entrer.
2. Forkez, créez une branche depuis `main` (`feat/...`, `fix/...`, `docs/...`).
3. Commits en anglais, à l'impératif, un changement logique par commit.
4. Vérifiez localement avant de pousser :

   ```bash
   cargo fmt --check
   cargo clippy -- -D warnings
   make check-arch
   cargo test
   ```

5. Ouvrez la PR vers `main`. Décrivez le *pourquoi*, pas seulement le *quoi* ;
   liez l'issue ; signalez tout choix de conception discutable plutôt que de
   le laisser découvrir en revue.
6. Une PR = un sujet. Les PR fourre-tout (refactoring opportuniste + feature +
   reformatage) seront renvoyées découpées.

## Revue et merge

- Les mainteneurs de Wedge Digital assurent la revue. Le délai visé est de
  quelques jours ; ce projet est maintenu sérieusement mais sans équipe dédiée.
- La revue porte d'abord sur la conformité architecturale, ensuite sur le code.
- Le merge est réservé aux mainteneurs. Pas de merge commit de courtoisie :
  une PR peut être refusée même après itérations si elle ne trouve pas sa place.

## Signalement de failles de sécurité

N'ouvrez **pas** d'issue publique pour une vulnérabilité. Écrivez à
<bertrand.begouin@wedge-digital.com> avec les détails et, si possible, un
scénario de reproduction. Vous recevrez un accusé de réception sous 72 h.

## Questions

Pour toute question sur ce processus, ouvrez une issue avec le label `question`
ou écrivez à <bertrand.begouin@wedge-digital.com>.

---

*Ce document peut évoluer ; la version faisant foi est celle de la branche `main`
au moment où vous ouvrez votre PR.*
