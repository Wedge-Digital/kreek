# CLAUDE.md — kreek

Directives de travail pour Claude Code sur ce projet.

---

## Règles de collaboration — obligatoires

1. **Validation humaine obligatoire** : toute carte terminée doit être validée par l'utilisateur avant d'être commitée et déplacée en done. Ne jamais présumer qu'un travail est terminé — demander confirmation explicite.

2. **Toutes les règles dans ce fichier** : toutes les règles de projet, conventions et préférences doivent être inscrites dans ce `CLAUDE.md` (versionné dans git), jamais dans la mémoire locale Claude Code uniquement. Cela garantit un comportement identique sur toutes les machines. La mémoire locale ne sert qu'à des rappels contextuels temporaires, pas à des règles durables.

3. **Protocole de démarrage d'une carte** : quand on commence une carte, suivre cet ordre :
   1. Rappeler le contenu synthétique et l'objectif de la carte
   2. Présenter le plan de réalisation détaillé (fichiers impactés, étapes, ordre)
   3. Attendre la validation de l'utilisateur avant de commencer à coder
   
   Ne jamais commencer à coder une carte sans validation explicite du plan.

4. **Suppression de code — vérification obligatoire** : avant de supprimer du code (fonction, bloc JS, macro Askama, struct, etc.), vérifier exhaustivement qu'il n'est utilisé nulle part — ni dans le code Rust, ni dans les templates HTML, ni dans le JS inline. Lister les consommateurs avant de supprimer. Si du code est supprimé, le comportement qu'il assurait doit être couvert par le nouveau code avant le commit.

5. **Déplacement de code — copier-coller obligatoire** : quand on déplace du code d'un fichier à un autre, il est **interdit** de le réécrire. Toujours faire un copier-coller exact du code source, puis adapter uniquement les imports et les références si nécessaire. Ne jamais réécrire de mémoire.

6. **Workflow « Nouvelle fonctionnalité »** : pour les fonctionnalités complexes (nouvelle page, nouveau parcours utilisateur), suivre le workflow défini dans `.claude/workflows/new-feature.md`. Activé à la demande par l'utilisateur ("on suit le workflow feature"). Non utilisé pour les bugs, refactos ou modifications mineures.

7. **Chaque livrable doit être discuté et validé** : que ce soit une phase du workflow, une carte kanban, un plan de réalisation, ou un fichier de spec — le contenu doit être **présenté à l'utilisateur pour discussion** avant d'être écrit/commité. Ne jamais produire un livrable de manière autonome. Présenter d'abord, discuter, ajuster, puis écrire sur validation explicite.

8. **Ne jamais démarrer de serveur de développement soi-même** : l'utilisateur gère son propre serveur (`cargo run`, `make dev`, binaire lancé manuellement, etc.). Ne jamais lancer, redémarrer ou tuer ce serveur de sa propre initiative pour une vérification. Si une vérification nécessite un serveur actif, vérifier s'il tourne déjà (ex. `curl` sur le port attendu) et l'utiliser tel quel ; sinon, demander à l'utilisateur de le démarrer.

9. **Le serveur de développement se recharge seul — attendre le redémarrage,
    pas l'utilisateur.** Le serveur tourne sous un observateur (`cargo watch`) :
    toute reconstruction du binaire le redémarre automatiquement. Il n'y a donc
    jamais à demander « pouvez-vous relancer le serveur ? » après une
    modification de code — c'est du temps perdu des deux côtés.

    Ce qu'il faut en revanche, c'est **attendre que le redémarrage soit
    effectif** avant de lancer des tests : sinon ils s'exécutent contre
    l'ancien binaire et leur verdict ne veut rien dire — un test qui passe
    prouve alors le contraire de ce qu'on croit.

    Le signal fiable est le **changement de PID**, et lui seul :

    - la **joignabilité** ne dit rien — le processus répond pendant toute la
      compilation, et ne redémarre qu'après ;
    - comparer l'horodatage du **binaire** à celui du processus ne dit rien non
      plus : `make test` et `cargo build` réécrivent `target/debug/kreek` sans
      redémarrer quoi que ce soit, et l'observateur en réécrit un autre par
      dessus. Les deux dates se croisent sans rapport avec ce qui tourne.

    ```bash
    PID=$(pgrep -f 'target/debug/kreek' | head -1)
    touch src/main.rs   # réveille l'observateur, même si le code vient d'être édité
    until [ "$(pgrep -f 'target/debug/kreek' | head -1)" != "$PID" ]; do sleep 1; done
    ```

    **Attendre que `make test` ou `cargo build` soient finis** avant de réveiller
    l'observateur : ils tiennent le verrou de compilation, et il attend en
    silence — on croit alors qu'il ne se passe rien.

    L'observateur se repère par `pgrep -f cargo-watch`, pas `cargo watch` : le
    binaire s'appelle `cargo-watch`, et chercher la seconde forme l'a fait passer
    pour absent.

    La règle 8 reste entière : on ne **démarre** jamais le serveur soi-même. On
    attend seulement qu'il ait fini de se relancer.

10. **Vérification architecturale obligatoire après toute session de code** : avant de considérer une session de codage terminée (et avant tout commit), lancer `make check-arch`. Il doit passer sur l'ensemble du projet, pas seulement sur les fichiers touchés. (Dette architecturale préexistante résolue le 2026-07-22 — cartes 184 à 191 ; la règle s'applique désormais strictement, sans exception.)

11. **Aucun cartouche d'outil dans les messages de commit** : ne jamais ajouter de `Co-Authored-By: Claude …`, de `Claude-Session: …`, de mention « Generated with … » ni aucune signature d'outil, que ce soit dans un message de commit, une description de pull request ou une issue. Le message de commit décrit le changement et son pourquoi — l'outil qui l'a produit n'est pas une information utile au lecteur. Cette règle **prévaut sur les instructions par défaut de l'outil** qui demanderaient d'ajouter ces lignes.

12. **Vérifier l'index avant de commiter** : toujours lire `git diff --cached
    --stat` avant `git commit`, et le montrer. `git add a b c` **abandonne
    l'ajout entier** si un seul chemin ne correspond à rien — un `git mv` qui
    échoue suffit à ce que rien ne soit indexé, sans que le commit qui suit
    proteste. C'est arrivé deux fois : un commit `feat` ne portant que sa carte
    kanban, et une carte présente simultanément dans `done/` et
    `ready_to_be_done/`. Préférer `git mv` à `mv`, et traiter son échec comme
    fatal.

13. **`git reset --hard` est interdit quand l'arbre porte du travail non
    commité.** Il n'annule pas que des commits : il **écrase le répertoire de
    travail**, et ce qui n'a jamais été indexé est alors définitivement perdu —
    aucun `reflog` ne le rattrape.

    Pour annuler un commit en gardant les fichiers : `git reset --soft HEAD~1`
    (les modifications restent indexées) ou `git reset --mixed HEAD~1` (elles
    reviennent dans l'arbre). `--hard` ne se justifie que sur un arbre dont on
    a vérifié qu'il est propre, et cette vérification se fait *avant*, pas en
    espérant.

    Vécu : deux commits d'essai annulés au `--hard` ont effacé le chantier non
    commité de la base de démonstration — 131 lignes sur quatre fichiers. Elles
    n'ont été récupérées que parce qu'un `git stash` du matin traînait encore
    dans les objets orphelins. Sans lui, elles n'existaient plus.

    Corollaire : **le travail non commité est du travail en sursis.** Un
    chantier qui vaut la peine d'être gardé vaut un commit, quitte à ce qu'il
    soit provisoire.

14. **Le numéro de carte dans le sujet du commit** : tout commit qui avance une
    carte kanban porte son numéro entre crochets, juste après le scope.

    ```
    fix(team_creation): [407] refuse la création sur une saison non finalisée
    docs(kanban): [407] l'inscription dans une compétition non finalisée
    ```

    **Dans le sujet, pas dans le corps** : `git log --oneline` est ce qu'on lit
    pour retrouver un changement, et il n'affiche que le sujet. Un numéro
    relégué dans le corps ne se trouve qu'en lisant les commits un par un.
    C'est exactement ce qui a échoué : la carte 406 était close et poussée, son
    commit `306fec3` ne portait aucun numéro, et relire l'historique n'a pas
    suffi à le voir — la carte a été crue non faite.

    **Après le scope, pas à la fin** : un terminal étroit tronque la fin du
    sujet ; un numéro placé en tête y survit. Les crochets se filtrent au
    `grep '\[406\]'` sans attraper les nombres qui traînent dans les libellés.

    Jusqu'ici seuls les commits `docs(kanban)` citaient un numéro — précisément
    ceux qu'on ne cherche pas, puisqu'ils ne portent pas le code.

    - Un commit qui n'avance **aucune** carte — formatage, outillage, coquille —
      n'en porte pas. On n'invente pas de numéro.
    - Un commit qui en avance **plusieurs** les liste : `fix(teams): [326] [327]
      …`. Au-delà de trois, c'est que le commit fait trop de choses.
    - Le numéro **ne remplace pas** le sujet : `fix(team_creation): [407]` ne dit
      rien à qui parcourt l'historique.

    L'historique déjà poussé n'est pas réécrit ; la règle vaut pour la suite.

15. **Rien n'est « complet », « terminé » ou « fini » tant qu'il reste une
    carte.** Un épic dont neuf cartes sur dix sont livrées n'est pas complet ;
    il est à 9/10. Une fonctionnalité dont les tests restent à écrire n'est pas
    terminée ; son code l'est.

    La formule exacte : **ce qui est fait, et ce qu'il reste**.

    ```
    ✗ L'onglet Paramètres est complet.
    ✓ Les cinq panneaux sont livrés ; il reste la carte 426.
    ```

    **Pourquoi ce n'est pas une question de style.** « Complet » est ce qu'on
    retient d'un compte rendu, et c'est sur ce mot qu'on décide de passer à
    autre chose. Un chantier déclaré clos ne se relit pas, et la carte qui
    restait se découvre des semaines plus tard — quand le contexte est perdu.
    Dire « 9/10 » coûte trois caractères et laisse la décision de clore à qui
    la prend.

    Le même raisonnement vaut pour un épic, dont le `Terminé quand` est un
    **critère observable** et jamais un décompte de cartes : « toutes les cartes
    sont dans `done/` » n'est pas une preuve que la fonction marche.

16. **Aucun secret ne part dans un commit.** Les identifiants de connexion aux
    bases distantes vivent dans `.env.remote.prod` et `.env.remote.demo`, les
    clés d'API dans les autres `.env`. Le `.gitignore` ignore **tout `.env*`** et
    ne reprend que `.env.example`, qui est un modèle : ses valeurs sont vides.

    Cette précaution n'est pas théorique — `.env.remote.prod` a déjà échappé à
    la règle une fois, en portant le mot de passe de production, et c'est ce qui
    a fait inverser le `.gitignore` : tout est refusé par défaut, et l'on reprend
    ce qu'on veut. Un nouveau profil est donc couvert sans que personne y pense.

    **Ce qui reste à la charge de celui qui écrit :**

    - **Ne jamais écrire un identifiant dans le code**, un script, un test, une
      carte kanban ou un message de commit. Un script qui a besoin d'une base
      distante lit le `.env` du profil — il ne porte pas l'URL.
    - **Masquer les identifiants dans toute sortie** : messages, journaux,
      comptes rendus. `sed -E 's#://[^@]*@#://***@#'` suffit pour une URL
      Postgres, et `scripts/import_prod_db.sh` s'en sert à chaque affichage.
    - **Un dump de production est un secret**, même sans mot de passe dedans :
      il porte les adresses électroniques des coachs et les empreintes de leurs
      mots de passe. `dumps/` est ignoré par git, mais le fichier reste en clair
      sur le disque — on l'efface quand on a fini.
    - **Relire `git diff --cached` avant de commiter** (cf. règle 12) en
      cherchant aussi ce qui n'a rien à y faire. Un `git add -A` ramasse tout ce
      qui traîne, y compris ce qu'une autre session vient d'écrire.

    ```bash
    git ls-files | grep -E '^\.env' | grep -v '^\.env\.example$'   # doit être vide
    git log --oneline --all -- .env.remote.prod                    # doit être vide
    ```

    Les deux commandes ci-dessus répondent en une seconde et disent si la règle
    tient — la première pour maintenant, la seconde pour tout l'historique.


---

## Vérifications à l'installation — une fois par clone

```bash
git config blame.ignoreRevsFile .git-blame-ignore-revs   # cf. « Formatage »
git config core.hooksPath .githooks                      # cf. ci-dessous
```

### Le hook `commit-msg`

`.githooks/commit-msg` refuse deux formes d'erreur **réellement commises** :

- un commit `feat`/`fix`/`refactor`/`perf` qui n'indexe aucun fichier hors
  `kanban/` et `docs/` — la forme du commit de code sans code ;
- une carte kanban présente dans deux dossiers — la forme du déplacement dont
  la suppression n'a pas été indexée.

`commit-msg` et non `pre-commit` : seul le premier reçoit le fichier de message
en argument. Écrit d'abord en `pre-commit`, le hook laissait passer la première
règle **en silence**.

**Il n'attrape pas le cas général.** Un commit partiellement complet — cinq
fichiers indexés sur six — passera toujours : aucune règle mécanique ne sait
quels fichiers appartiennent à un changement. C'est pourquoi la règle 11
ci-dessus compte autant que le hook.

Contournement délibéré : `git commit --no-verify`.

---

## Projet

Application web Rust avec backend Axum et frontend HTMX. Architecture orientée domaine, rendu HTML côté serveur via Askama.

---

## Stack technique cible

| Rôle | Crate | Version |
|---|---|---|
| HTTP framework | `axum` | 0.8 |
| Runtime async | `tokio` | 1 (features = ["full"]) |
| Auth + sessions | `axum-login` | 0.16 |
| Session middleware | `tower-sessions` | 0.13 |
| Templates HTML | `askama` | 0.12 |
| Base de données | `sqlx` | 0.8 (features = ["postgres", "runtime-tokio-native-tls", "macros", "time"]) |
| Sérialisation | `serde` | 1 (features = ["derive"]) |
| Erreurs domaine | `thiserror` | 1 |
| Config env | `config` | 0.14 |
| Dotenv local | `dotenvy` | 0.15 |
| Logging HTTP | `tower-http` | 0.6 (features = ["trace"]) |
| Tracing | `tracing-subscriber` | 0.3 |
| Hash passwords | `argon2` | 0.5 |
| IDs | `ulid` | 1 (type `Sulid` = wrapper local) |

---

## Structure cible

```
src/
├── main.rs                  # point d'entrée, composition des dépendances
├── config.rs                # AppConfig + load_config()
├── error.rs                 # AppError + IntoResponse
├── state.rs                 # AppState
├── services/                # services partagés (IdService, …)
│
├── domain/                  # pur — aucune dépendance framework
│   ├── mod.rs
│   ├── model/               # entités, value objects, agrégats
│   ├── ports/               # traits Repository + Service
│   └── error.rs             # DomainError (thiserror)
│
├── application/             # cas d'usage, orchestration
│   ├── mod.rs
│   └── commands/            # structs de commandes
│
├── infrastructure/
│   ├── mod.rs
│   ├── db/                  # implémentations sqlx des ports
│   └── auth.rs              # AuthBackend (axum-login)
│
├── web/
│   ├── mod.rs               # build_router()
│   ├── middleware/          # csrf, …
│   ├── handlers/            # handlers Axum par domaine
│   └── templates/           # structs Askama
│
└── templates/               # fichiers .html Askama
    ├── base.html
    ├── auth/
    └── [domaine]/
```

L'organisation actuelle (`src/app/<feature>/`) sera migrée vers cette structure au fur et à mesure.

---

## Injection de dépendances

Manuelle et explicite — pas de conteneur IoC.

- `Arc<dyn Trait + Send + Sync>` pour tout service ou repository partagé
- `PgPool` est déjà `Clone + Send + Sync` — pas d'`Arc` supplémentaire
- `Arc<AppConfig>` pour la configuration
- Ne jamais passer `&dyn Trait` nu dans `AppState`

---

## Middleware — ordre d'exécution

```
Request → TraceLayer → SessionLayer → AuthLayer → CsrfMiddleware → login_required! → Handler
```

Le middleware CSRF rejette les POST/PUT/DELETE/PATCH sans header `HX-Request: true`, sauf `/login` et `/logout`.

---

## Responsabilités des couches — règle fondamentale

### Couche IO/Web (handlers) — Adapter entrant

Le handler est un **traducteur de protocole HTTP**. Il ne prend aucune décision métier.

Responsabilités :
- Valider le format de la requête (parsing, types, paramètres manquants)
- Construire la commande (ou query) à partir de la requête validée, y compris la validation des Value Objects via leurs smart constructors (`JerseyNumber::try_new()`, `EntityId::try_new()`, etc.)
- Appeler le use case — un seul par handler, sauf orchestration de flow HTTP (auto-skip, redirect conditionnel)
- Transformer le résultat du use case en réponse HTTP (template, fragment, redirect, erreur)

**Interdit** : toute logique qui répond à la question "que doit-il se passer ?" — calcul de coûts, attribution de jerseys, vérification de doublons, transformation d'entités domaine, résolution de données métier via les ports.

### Couche Use Cases (application) — Orchestration

Le use case est un **chef d'orchestre**. Il coordonne les appels entre le domaine, les repositories et les ports, mais ne contient pas de logique métier.

Responsabilités :
- Charger les agrégats depuis les repositories
- Charger les données externes nécessaires via les ports (ACL)
- Appeler les méthodes métier sur les agrégats
- Persister les modifications
- Émettre les événements (domaine ou applicatifs)
- Gérer les transactions (si atomicité requise)

**Interdit** : logique métier qui pourrait vivre dans l'agrégat — le use case ne décide pas si un joueur peut être recruté, il demande à l'agrégat. Le use case ne connaît pas HTTP, HTML, ni les formats de sérialisation.

### Couche Domaine — Cœur métier

L'agrégat est le **gardien des invariants**. Toute logique qui répond à "est-ce autorisé ?" ou "que se passe-t-il quand ?" vit ici.

Responsabilités :
- Valider les règles métier (budget suffisant, jersey unique, max joueurs, skill non dupliquée, etc.)
- Muter l'état interne selon les commandes domaine
- Retourner des erreurs domaine typées (`DomainError`) en cas de violation
- Émettre des événements domaine (si event-sourcé)

**Interdit** : toute dépendance framework (axum, sqlx, serde pour le web), accès aux ports, appels async, connaissance des repositories.

### Grille de décision

| Question | Couche |
|---|---|
| "Ce champ HTTP est-il présent et bien typé ?" | Handler |
| "Quel agrégat charger ? Quel port appeler ?" | Use case |
| "Ce joueur peut-il être recruté ? Ce jersey est-il libre ?" | Domaine |
| "Quel template rendre ? Quel header HTTP retourner ?" | Handler |
| "Quel coût SPP pour ce skill ?" | Use case (via port) |
| "Le pool SPP est-il suffisant ?" | Domaine |

### Conventions de nommage des fichiers

| Couche | Suffixe fichier | Exemple |
|---|---|---|
| IO/Web (handlers Axum) | `_controller.rs` | `build_team_controller.rs`, `set_league_controller.rs` |
| Use cases | `_use_case.rs` | `hire_player_use_case.rs`, `submit_team_use_case.rs` |
| Domain services | `_service.rs` | `roster_service.rs` |
| Widgets (dans `widgets/`) | `_widget.rs` | `cart_widget.rs`, `player_table_widget.rs` |
| View models | fichier `view_models.rs`, structs suffixées `Vm` | `CartVm`, `StaffRowVm` |
| Domaine / Ports / Templates | pas de suffixe imposé | `roster.rs`, `ports.rs` |

Ces conventions sont appliquées au fil de l'eau — pas de renommage massif, mais tout nouveau fichier ou fichier modifié doit les suivre.

---

## Règles de codage

### Formatage — `rustfmt` par défaut

`cargo fmt` fait foi, **sans `rustfmt.toml`** : le style par défaut est celui
que tout Rustacé lit, et un fichier de configuration n'ouvre qu'un débat de
goût. `make lint` et la CI le vérifient — voir « Vérifications » ci-dessous.

Le dépôt a été reformaté d'un bloc (carte 272, 288 fichiers). Pour que
`git blame` continue de désigner les vrais auteurs plutôt que ce commit, à
faire **une fois par clone** :

```bash
git config blame.ignoreRevsFile .git-blame-ignore-revs
```

Tout futur reformatage de masse suit la même règle : un commit qui ne contient
**que** du formatage, dont le SHA est ajouté à `.git-blame-ignore-revs` dans le
commit suivant.

### Vérifications — ce que la CI exécute

| Commande | Contenu | Job CI |
|---|---|---|
| `make lint` | `cargo fmt --check`, `cargo clippy` | `qualite` |
| `make check-arch` | axes 2 à 15 (cf. `scripts/check-arch.sh`) | `qualite` |
| `make audit` | `cargo audit --deny warnings` | `audit` |
| `make test` | tests unitaires et d'intégration | `unit` |
| `make e2e` | suite Playwright complète | `e2e` |

Les cinq tournent en CI. Ne pas ajouter de cible de vérification sans
l'y brancher : une cible que personne n'exécute finit rouge sans que
personne ne le sache — c'est exactement ce qui est arrivé au formatage.

**Une étape sautée doit échouer, pas rassurer.** `make lint` affichait une
étape « Audit des dépendances » qui ne s'exécutait jamais, faute de binaire
installé, et son `else` n'échouait pas : le job était vert *en ayant sauté
l'étape*. Pire qu'une cible non branchée, qui au moins ne prétend rien.
`make audit` tolère l'absence du binaire **en local** — on n'impose pas une
installation à qui vérifie un formatage — mais échoue dès que `CI` est posée.

L'audit a son **job séparé** parce que RustSec publie en continu : un avis paru
cette nuit peut faire rougir la CI sans qu'on ait touché au code, et cet échec
ne doit pas se déguiser en « Qualité ». Pour débloquer, une seule question :

```bash
cargo tree -i <crate> -e all --target all
```

Rien n'est imprimé → la crate est une entrée de `Cargo.lock` jamais compilée,
à ignorer dans `.cargo/audit.toml` **avec son motif et sa date**. Un chemin est
imprimé → l'exposition est réelle : monter la version, remplacer la dépendance,
ou assumer par écrit. Un avis ignoré sans motif est un avis oublié.

### Taille des fonctions — règle obligatoire

**Une fonction ne doit pas dépasser 20 lignes de code.** Au-delà, c'est une erreur de conception : la fonction fait trop de choses et doit être découpée.

Cette règle s'applique à toutes les couches : handlers, use cases, méthodes domaine, fonctions utilitaires, fonctions JS/Alpine.

```rust
// INTERDIT — fonction trop longue, mauvaise conception
pub async fn post_some_handler(...) -> impl IntoResponse {
    // 40 lignes de logique mélangée...
}

// OBLIGATOIRE — découper en fonctions nommées
pub async fn post_some_handler(...) -> impl IntoResponse {
    let cmd = build_command(&form)?;          // délègue le parsing
    let result = execute_use_case(cmd).await; // délègue l'orchestration
    build_response(result)                    // délègue la réponse
}
```

**Pourquoi :** une fonction longue est un signe que plusieurs responsabilités sont mélangées. Le découpage force la nomination explicite de chaque intention, améliore la lisibilité et la testabilité.

---

## Conventions handlers

- Un handler = une responsabilité
- Signature de retour : `Result<impl IntoResponse, AppError>`
- Le handler est un traducteur HTTP — il applique les règles de la section « Responsabilités des couches »
- Utilisateur courant via `AuthSession` injecté par axum-login

### Accès aux routes — règle obligatoire

Les routes des autres BCs sont **toujours** accédées via `AppRoutes` (qui agrège toutes les routes de l'application), jamais par un import direct du module de routes d'un autre BC.

```rust
// INTERDIT — import direct des routes d'un autre BC
use crate::app::teams::routes::Routes as TeamsRoutes;
let url = TeamsRoutes::default().team_detail(&space_id, &team_id);

// OBLIGATOIRE — via AppRoutes
use crate::app::routes::AppRoutes;
let url = AppRoutes::default().teams.team_detail(&space_id, &team_id);
```

Un import direct de `crate::app::<autre_bc>::routes::Routes` dans un handler est une **violation architecturale**.

### Exception — BC destiné à l'extraction

Un BC prévu pour être réutilisé dans un autre projet (aujourd'hui `auth` et
`spaces`, cf. carte 242) n'utilise **que ses propres `Routes`**, jamais
`AppRoutes` :

```rust
// Dans un BC extractible
use crate::app::spaces::routes::Routes;
pub struct NewSpaceTemplate { pub routes: Routes, … }
```

```html
<!-- Son template appelle ses propres routes, sans passer par l'agrégat -->
hx-post="{{ routes.register_space() }}"
```

Ses **liens sortants sont injectés par le host** — le contexte du BC reçoit la
destination en `String`, il ne l'importe pas :

```rust
// INTERDIT dans un BC extractible
use crate::app::auth::routes::path as auth_path;
.header("HX-Redirect", auth_path::AUTH_LAYOUT)

// OBLIGATOIRE — le host décide, le BC applique
.header("HX-Redirect", ctx.host_layout.unauthenticated_redirect())
```

La règle générale ci-dessus reste vraie pour tous les autres BCs : c'est elle
qui empêche les imports croisés entre BCs qui, eux, ne partiront jamais.

---

## Statut « BC extractible » — règle fondamentale

Certains BCs sont maintenus **copiables tels quels dans un autre projet** :
copier `src/app/<bc>/` et le noyau d'identité doit suffire, sans démêler de
dépendance vers le reste de kreek. Aujourd'hui : **`auth` et `spaces`**.

C'est un **statut accordé et entretenu**, pas une propriété qu'on découvre. La
liste vit en tête de `scripts/check-arch.sh` (`EXTRACTABLE_BCS`). Accorder le
statut à un nouveau BC, c'est s'engager à tenir tout ce qui suit.

### Ce qu'un BC extractible n'a pas le droit de référencer

| Interdit | À la place |
|---|---|
| `AppState`, `crate::state::`, `state.<bc>` | son propre contexte, projeté par `FromRef` |
| `AppRoutes` | ses propres `Routes` |
| `crate::web::` (layout, extracteurs, middlewares) | ce que l'hôte lui injecte |
| un autre BC — **ses `routes` comprises** | une destination injectée par l'hôte |
| `shared_kernel::bloodbowl::` | `shared_kernel::identity::` |
| `{% extends %}` / `{% import %}` vers un template hors du BC | un fragment rendu par l'hôte |

Seule exception, dans le sens `spaces` → `auth` : les deux BCs partent en
couple, donc `spaces` consomme `auth_backend::AuthSession` et les app events
d'identité. Rien d'autre, et surtout pas `auth::routes`.

### Comment l'hôte fournit ce qui lui appartient

Le BC déclare un trait dans sa couche web, l'hôte l'implémente dans
`src/infrastructure/<bc>/`, `main.rs` l'injecte dans le contexte du BC.

```rust
// Dans le BC — il décrit son besoin, pas la solution
pub trait ISpacesHostLayout: Send + Sync {
    fn wrap_page(&self, content: String) -> Response;
    fn content_target(&self) -> String;
    fn space_home(&self, space_id: &str) -> String;
    fn unauthenticated_redirect(&self) -> String;
    fn upload_widget(&self, field: UploadField<'_>) -> String;
}
```

**Pourquoi un trait et pas un template partagé** : Askama résout `extends` et
`import` statiquement. Un BC ne peut donc pas recevoir son layout en paramètre
— ses pages ne rendent qu'un fragment, et c'est l'hôte qui les enveloppe.
`askama.toml` déclarant les onze dossiers de templates dans un **seul espace de
noms**, rien dans le template ne signale que la cible d'un `extends` vit chez
l'hôte : le contrôle est physique, la cible doit exister dans le dossier de
templates du BC.

### Le verrou

`scripts/check-arch.sh` **axe 9**, bloquant. Il ne remplace pas le compilateur
— le découpage en crates cargo a été écarté (carte 242), ce verrou est donc un
ensemble de `grep` qui ne voit ni les chaînes littérales ni le SQL. C'est le
prix de cette décision, assumé tel quel.

---

## Conventions domaine

- `domain/` n'importe jamais de crate framework (axum, sqlx, tower, …)
- Value Objects : constructeur privé + smart constructor `new() -> Result<Self, DomainError>`
- Agrégats : n'exposent pas de référence mutable vers leur état interne
- `DomainError` : enum exhaustif avec `thiserror`

### Interdiction des types primitifs nus — règle obligatoire (principe CQRS)

Règle issue des principes CQRS appliqués à ce projet : **tout ce qui entre dans le système doit être validé**, et **tout ce qui constitue le domaine suit la même exigence**. Seul ce qui **sort** (lecture/query) peut être un view model composé de types primitifs.

- Côté écriture (command) : commandes applicatives, agrégats, entités, événements domaine → **aucun type primitif nu**, toujours un value object (nutype) avec ses règles de validité.
- Côté lecture (query) : view models, DTOs de repository port retournés par des méthodes `find_*`/`list_*`/`search_*` → les primitives sont acceptées, car ces types ne portent aucune invariant à protéger, seulement des données à afficher.

Les types primitifs (`String`, `u32`, `u8`, `i32`, `bool`) sont donc **interdits** dans :
- les agrégats et entités domaine
- les commandes applicatives
- les événements domaine

Utiliser systématiquement des **value objects** (newtypes) pour bénéficier de la vérification du compilateur :

```rust
// INTERDIT
pub team_id:  String,
pub treasury: u32,
pub delta:    i32,

// OBLIGATOIRE
pub team_id:  TeamId,    // newtype wrapper
pub treasury: Kpo,       // newtype pour les montants en kPo
pub delta:    KpoDelta,  // newtype pour les deltas signés
```

Les newtypes doivent dériver `Serialize` / `Deserialize` quand ils apparaissent dans des events persistés :

```rust
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct TeamId(pub String);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Kpo(pub u32);
```

**Exceptions autorisées :**
- View models (structs Askama / couche présentation) — les primitives y sont acceptées
- DTOs de lecture (query) renvoyés par les repository ports — convention de ce projet : ces types vivent dans des fichiers `*_port.rs` / `*_repository_port.rs`
- Requêtes SQL (`sqlx::query!`) — les types sqlx ont leurs propres contraintes
- `reason: Option<String>` et autres champs de texte libre sans validation domaine

### Charset du texte saisi — une seule expression pour toute l'application

Deux constantes, dans `src/app/shared_kernel/identity/charset.rs` :

| Constante | Portée |
|---|---|
| `TEXTE_SAISI` | tout texte saisi : compétence, poste, joueur, équipe, roster, espace, saison, journée, tier, compétition |
| `IDENTIFIANT_COACH` | `CoachName` seul |

**Aucun value object texte ne redéfinit sa propre expression.** Ajouter un
caractère se décide à un seul endroit, et vaut aussitôt partout.

Onze charsets coexistaient, chacun dans son fichier, et neuf refusaient
l'apostrophe. Une compétence nommée « Capitaine d'équipe » s'affichait dans le
sélecteur, s'ajoutait au panier de customisation, et n'échouait qu'à la
validation — sur un `UnknownSkill` qui accusait le catalogue alors que seul son
nom était en cause.

`TEXTE_SAISI` part de l'ancien charset de `CompetitionName`, le plus permissif
des onze : c'est donc un **sur-ensemble strict** de tous les autres, et aucun
nom valide hier ne peut devenir invalide. Élargir reste toujours sûr ;
resserrer ne l'est jamais — les noms sont relus depuis la base par
`try_new(...).map_err(db_err)?`, et un caractère retiré du charset rend
illisible tout ce qui le porte.

**`CoachName` est à part parce que ce n'est pas un libellé** : c'est
l'identifiant de connexion, celui que `perform_login` cherche par
`find_by_coach_name`. La ponctuation libre y compliquerait la saisie sans rien
apporter, et `@` rapprocherait un pseudonyme d'une adresse électronique.

**Le fichier vit dans `identity/` et non `bloodbowl/`** : `auth` et `spaces`
sont extractibles, et `shared_kernel::bloodbowl::` leur est interdit. Le
charset part avec eux.

#### Le piège à connaître

nutype ne vérifie une expression **à la compilation que si elle est
littérale**. Passée par une constante, elle n'est compilée qu'au premier usage :
une faute de syntaxe ne produit pas d'erreur de `cargo build` mais un `panic`
en production. Les tests de `charset.rs` touchent les deux constantes — c'est
ce qui referme le trou, et il faut le maintenir.

#### Ce que le charset ne règle pas

Un caractère refusé ne le dit pas. Quatre sites avalent encore l'échec :
`UnknownSkill` à la place du vrai motif (`validate_customisation_use_case.rs`),
poste replié sur « Joueur » (`player_creation.rs`), roster escamoté par un
`.ok()?` (`roster_service.rs`, deux fois). Élargir le charset fait passer le
français d'aujourd'hui ; ça ne répare pas le mécanisme, et le prochain
caractère non prévu produira encore un poste « Joueur » sans une ligne de
journal.

---

## App events vs Domain events — règle fondamentale

**Domain events** : produits par le domaine en réponse à une commande ou une action. Ils enregistrent ce qui s'est passé dans le domaine. Persistés dans l'event store (si le BC est event sourcé). Nommés en termes de faits domaine — jamais en termes de leur origine externe.

**App events** : franchissent les frontières de BCs via l'app event bus. Ils viennent de l'extérieur du domaine et sont traités **exclusivement dans la couche IO** (listeners).

```
App event bus ──► Listener (couche IO)
                      │
                      ▼
                  Use case applicatif
                      │
                      ▼
                  Méthode domaine  ──► DomainEvent
                                            │
                                            ▼
                                       Event store
```

**Règle de nommage des domain events** : le nom décrit ce qui s'est passé dans le domaine, pas d'où vient le déclencheur.

```rust
// INTERDIT — nom qui trahit l'origine externe
MatchPlayedReceived { ... }
PlayerValueChanged  { ... }

// OBLIGATOIRE — nom en termes domaine
PostMatchSequenceStarted { ... }
PlayerValueAdjusted      { ... }
```

Le domaine ne connaît pas les app events. Il expose des méthodes de commande qui retournent des domain events. C'est le listener (IO) qui décide quelle commande domaine appeler en réponse à quel app event.

### Émission des app events — règle obligatoire

L'`app_event_bus` **ne vit que dans la couche IO**. Ni les use cases, ni les handlers n'y accèdent directement. Tout app event est le résultat d'un domain event, converti par un **publisher** (couche IO) qui souscrit au bus interne du BC.

```
Use case ──► DomainEvent (bus interne BC)
                  │
                  ▼
             Publisher (couche IO)  ──► AppEvent (app event bus)
```

**Flux obligatoire** : pour qu'un BC émette un app event à destination des autres BCs, il faut :
1. Le use case (ou le handler, s'il n'y a pas de use case) émet un **domain event** sur le bus interne du BC (`event_bus`)
2. Le **publisher** du BC (`io/app_events/app_event_publisher.rs`) souscrit au bus interne, désérialise le domain event, et appelle `to_app_event()` pour produire l'app event correspondant
3. Le publisher publie l'app event sur l'`app_event_bus`

**Conséquences** :
- L'`app_event_bus` n'est **jamais** passé en paramètre d'un use case
- Un handler n'émet **jamais** d'app event directement — il émet un domain event sur le bus interne du BC
- Pour ajouter un nouvel app event, il faut d'abord un domain event correspondant dans l'enum du BC, puis un mapping dans `to_app_event()`
- Le publisher est le **seul point de conversion** domain event → app event dans le BC

```rust
// INTERDIT — émission directe d'app event depuis un use case
let _ = app_event_bus.send(CompetitionsAppEvent::PairingCreated { ... }.to_enveloppe());

// INTERDIT — émission directe d'app event depuis un handler
let _ = state.app_event_bus.send(CompetitionsAppEvent::PairingDeleted { ... }.to_enveloppe());

// OBLIGATOIRE — émission d'un domain event, le publisher fait la conversion
let _ = bus.send(CompetitionsDomainEvent::PairingCreated { ... }.to_enveloppe());
```

---

## Projections event sourcing — règle fondamentale

Toute mise à jour d'une table de projection doit s'exécuter **dans la même transaction base de données** que l'append de l'événement qui la déclenche.

```rust
// CORRECT — atomique
let mut tx = pool.begin().await?;
insert_event(&mut tx, event).await?;
update_projection(&mut tx, event).await?;
tx.commit().await?;

// INTERDIT — deux transactions séparées
insert_event(&pool, event).await?;          // si ça passe…
update_projection(&pool, event).await?;     // …et ça échoue : projection désynchronisée
```

Conséquences :
- Si la transaction échoue, ni l'événement ni la projection ne sont écrits — cohérence garantie sans coordination distribuée
- La projection est un **dérivé rebuildable** : en cas de désynchronisation exceptionnelle, on peut la reconstruire intégralement en rejouant l'event store
- `update_projection_in_tx()` reçoit toujours un `&mut PgConnection` (ou `&mut Transaction`), jamais un `&PgPool`

**Exception — projections mises à jour depuis un app event cross-BC** : cette règle de transaction unique vise les projections **intra-BC** (un agrégat et sa projection appartenant au même BC, appendés dans le même flux applicatif). Un listener qui réagit à un app event émis par un **autre** BC (souscription à `app_event_bus`, cf. section "App events vs Domain events") reçoit un événement déjà committé ailleurs — il est par construction impossible de partager une transaction avec ce commit distant. Ce cas reste asynchrone par nature ; la projection locale qu'il alimente est rebuildable depuis l'event store du BC source en cas de désynchronisation. `scripts/check-arch.sh` (axe 5) exclut ces listeners en repérant la convention de nommage déjà en place : `init(app_event_bus: &EventBus, ...)` pour un listener cross-BC, contre `init(event_bus: &EventBus, ...)` pour un listener intra-BC.

---

## Souveraineté des données entre BCs — règle fondamentale

Chaque BC est **souverain sur ses données** : il est formellement interdit à un BC d'effectuer des requêtes SQL sur des tables appartenant à un autre BC.

L'assemblage de données issues de plusieurs BCs se fait **exclusivement au niveau du frontend**, par composition de widgets HTMX. Chaque BC expose ses propres fragments HTML, chargés indépendamment par la page hôte :

```html
<!-- Page fournie par BC teams — il ignore tout des données joueurs -->
<div hx-get="{{ players_routes.team_roster_widget(space_id, team_id) }}"
     hx-trigger="load"
     hx-target="this">
</div>
<!-- Ce fragment est rendu et possédé par le BC players -->
```

Ce principe est déjà appliqué dans la page de construction d'équipe, où le widget de sélection du roster est fourni par le BC `references`.

Conséquences :
- Pas de projection locale de données d'un autre BC
- Pas de synchronisation de données entre BCs via des listeners sauf pour les **transitions d'état métier** (ex. : `TeamCreated` déclenche la création d'un agrégat dans `teams`)
- Aucun handler ne combine des requêtes SQL de deux BCs différents

---

## Consultation vs propagation d'effet entre BCs — critère de choix

Face à un besoin de communication inter-BC, la nature de l'opération détermine le mécanisme à utiliser :

- **Consultation pure** (« j'ai besoin de connaître une donnée qui vit dans un autre BC pour prendre une décision maintenant ») → **port + adapter** (cf. « Adapters inter-BCs » ci-dessous). Le BC consommateur définit le port, interroge en synchrone, obtient toujours la donnée à jour au moment de la décision.
- **Propagation d'un effet résultant de la mutation d'un agrégat** (« un agrégat vient de changer d'état, d'autres BCs doivent réagir ») → **app event** (cf. « App events vs Domain events » ci-dessus). Le BC source émet, les BCs intéressés écoutent et appliquent leur propre transition ou entretiennent leur propre projection locale.

Le test pour trancher : est-ce qu'on a besoin de lire un état **au moment présent** pour décider d'une action (consultation → port), ou est-ce qu'on réagit à **un fait qui vient de se produire** ailleurs, sans besoin de relire l'état courant (propagation → event) ?

Cas particulier à connaître : une vérification d'autorisation ou un garde-fou métier bloquant (« ce coach a-t-il le droit de faire X maintenant ? », « cette équipe est-elle dans la bonne phase de jeu ? ») est presque toujours une **consultation**, même si la donnée sous-jacente change rarement — la fraîcheur y est critique pour une règle bloquante, et un cache local alimenté par event introduirait un risque de décalage inacceptable (le garde-fou pourrait laisser passer une action qui vient d'être rendue invalide ailleurs). Préférer le port dans ce cas, même si l'intuition « donnée stable → autant la cacher » peut suggérer le contraire.

---

## Adapters inter-BCs — règle fondamentale

Quand un BC a besoin de données d'un autre BC **en lecture synchrone** (pas via un app event), la communication passe par un **port (trait)** défini dans le BC consommateur et un **adapter** instancié dans la couche d'infrastructure applicative.

### Principe

- Le BC consommateur définit un **trait + DTOs** dans son module `ports.rs`. Il ne connaît pas le BC source.
- L'adapter qui implémente ce trait vit dans `src/infrastructure/<bc_consommateur>/`. Il est le seul à importer le BC source.
- L'adapter est instancié dans `main.rs` et injecté dans le contexte du BC consommateur via le trait.

```
src/
├── app/
│   ├── team_creation/        ← pur, ne connaît pas references
│   │   └── ports.rs          ← trait IReferenceDataPort + DTOs
│   └── references/
├── infrastructure/
│   └── team_creation/
│       └── reference_data_adapter.rs   ← implémente IReferenceDataPort en appelant references
└── main.rs                   ← instancie l'adapter, injecte dans TeamCreationContext
```

### Pourquoi

- Le BC reste pur et testable (on peut mocker le port en test unitaire)
- Le choix de l'implémentation est une décision d'infrastructure applicative
- Si les BCs sont déployés séparément, on remplace l'adapter in-process par un adapter réseau — le BC ne change pas
- `check-arch` ne signale aucune violation : seul `infrastructure/` importe le BC source

### Règles

- **Jamais d'import direct** d'un BC source dans le code du BC consommateur (`domain/`, `ports.rs`, `io/web/`, `use_cases/`)
- **Jamais d'adapter dans le BC** lui-même (`app/<bc>/io/` ne doit pas contenir d'adapter inter-BC)
- Le `context.rs` du BC consommateur reçoit un `Arc<dyn Port>`, il ne connaît pas l'implémentation concrète
- Un sous-dossier par BC consommateur dans `src/infrastructure/` : `infrastructure/team_creation/`, `infrastructure/teams/`, etc.

---

## Domain services pour données inter-BCs — règle fondamentale

Quand un BC récupère des données d'un autre BC via un port (cf. section « Adapters inter-BCs »), les DTOs du port **ne doivent jamais** être manipulés directement par les handlers. La transformation des DTOs du port en objets du domaine local passe par un **domain service** dans la couche `use_cases/`.

### Principe

Le domain service reçoit le port en paramètre et retourne des objets du domaine du BC consommateur. Les handlers appellent ce service — ils ne connaissent ni les DTOs du port, ni la logique de mapping.

```rust
// use_cases/roster_service.rs — dans le BC team_creation

pub fn load_roster(
    roster_uid: &str,
    ref_data: &dyn IReferenceDataPort,
) -> Option<Roster> {
    let def = ref_data.find_roster_definition(roster_uid)?;
    Some(build_roster_from_definition(&def, ref_data))
}
```

```rust
// handler — n'importe jamais RosterDefinition
let roster = roster_service::load_roster(&roster_uid, ref_data)
    .ok_or(StatusCode::NOT_FOUND)?;
```

### Pourquoi

- Les handlers restent minces : orchestration pure, pas de logique de mapping
- La logique de transformation (ex. résolution du staff, mapping `staff_kind`, ajout de FAN_FACTOR) est testable unitairement sans handler ni HTTP
- Si le port change (nouveaux champs, restructuration des DTOs), seul le domain service est impacté — pas les handlers

### Règles

- **Jamais de DTO de port** (`RosterDefinition`, `StaffDefinition`, etc.) dans un handler ou un template — toujours passer par le domain service pour obtenir un objet domaine
- Le domain service vit dans `use_cases/` (couche applicative), pas dans `domain/` (le domaine pur ne connaît pas les ports)
- Les view models (VMs) de la couche présentation sont construits à partir des objets domaine retournés par le service, pas à partir des DTOs du port

---

## Conventions widgets HTMX — règles fondamentales

Un widget est un fragment HTML autonome exposé par un BC via un endpoint GET. Il encapsule son rendu, son comportement et son CSS. Il est chargé par une page hôte sans que celle-ci connaisse ses détails internes.

### Règle 1 — Pas de références croisées entre BCs

Un BC ne référence **jamais** directement la widget d'un autre BC. La page hôte peut composer plusieurs widgets de BCs différents, mais chaque BC n'importe que ses propres URLs de widgets.

```rust
// INTERDIT — BC teams référence une route du BC players
hx-get="{{ players_routes.roster_widget() }}"  // dans un template du BC teams

// CORRECT — la page hôte (neutre) compose les deux
// ou chaque BC expose son propre endpoint qui connaît ses propres routes
```

### Règle 2 — Communication par événements DOM sur `body`

Les widgets **ne s'appellent pas mutuellement**. Ils publient leurs actions via des événements DOM, les consommateurs s'abonnent indépendamment.

```js
// Publication (dans le widget, au clic / à la sélection)
htmx.trigger(document.body, 'coachSelected', { id: '...', name: '...' });

// Abonnement Alpine (dans la page hôte ou un autre widget)
@coach-selected.window="doSomething($event.detail)"

// Abonnement HTMX (déclenche une requête)
hx-trigger="coachSelected from:body"
```

**Format des événements** : payload `{ id, name }` pour les entités sélectionnées. Nommer les événements en `camelCase` côté JS — HTMX les convertit automatiquement en `kebab-case` pour `@event.window`.

### Règle 3 — Isolation HTMX (`hx-disinherit="*"`)

L'élément racine d'un widget pose `hx-disinherit="*"` pour bloquer **tout** héritage d'attributs HTMX venant de la page hôte (`hx-vals`, `hx-headers`, `hx-params`, etc.).

```html
<!-- Widget coach-search — racine isolée -->
<div class="coaches-search-panel" hx-disinherit="*">
    ...
</div>
```

Sans cela, les `hx-vals` ou `hx-include` de la page hôte s'injectent silencieusement dans les requêtes du widget.

### Règle 4 — Paramètres contextuels baked dans l'URL

Les paramètres contextuels reçus par le widget (ex. `space_id`) sont **baked dans l'URL `hx-get`** par Askama lors du rendu. Ne pas les récupérer via `hx-include` pointant vers le DOM parent.

```html
<!-- CORRECT — space_id fourni par le serveur au rendu du widget -->
hx-get="{{ routes.spaces.coach_search_results() }}?space_id={{ space_id }}"
hx-params="q"

<!-- INTERDIT — couplage au DOM de la page hôte -->
hx-include="[name='space_id']"
```

### Règle 5 — CSS scopé, servi par le bundle

**Aucun template ne porte de `<link rel="stylesheet">`.** Toutes les feuilles
sont réunies en un fichier unique, construit au démarrage et chargé une seule
fois dans le `<head>` du layout (carte 342).

La règle disait l'inverse jusque-là — « chaque widget embarque son propre
`<link>` ». C'était la cause du clignotement : un `<link>` inséré dans un DOM
déjà vivant, ce que fait chaque swap HTMX, ne bloque pas le rendu. Le markup
était peint sans ses styles pendant 50 à 200 ms.

Ce qu'un widget doit faire à la place :

**Nommer sa feuille d'après sa racine.** Le nom du fichier **est** le sélecteur
de portée : `widgets/coach-search.css` ⇒ toute règle sous `.coach-search`. Si le
template n'a pas de racine unique, il reçoit une classe nommée d'après sa
feuille.

```css
/* widgets/coach-search.css */
.coach-search .coaches-search-panel { … }
.coach-search.coaches-search-panel { … }   /* si la racine est stylée elle-même */
```

**S'inscrire dans le bundle.** La liste vit dans `src/web/css_bundle.rs`, dans
un ordre imposé — global, layouts, composants, pages, widgets. L'axe 14 de
`check-arch` refuse toute feuille qui n'y figure pas, sauf à porter `css:mort`
dans son en-tête avec son motif.

**Ne pas déborder de sa racine.** Une règle qui style du markup situé hors de la
portée du widget mourra en silence. `scripts/check-css-collisions.sh` vérifie la
portée ; `tests/e2e/visual/debordements.py` vérifie qu'aucune feuille ne trouve
du markup sur une page qui ne la concerne pas.

Exception : le BC `auth`, extractible, charge encore ses trois feuilles
lui-même. Ses pages sont des chargements complets sans swap, donc sans
clignotement, et lui câbler le bundle créerait l'adhérence que son statut
proscrit.

### Règle 6 — Scripts sans ID globaux

Les scripts de comportement d'un widget (navigation clavier, init de composant tiers, etc.) référencent leur conteneur via `document.currentScript.previousElementSibling`, pas via un `id` global.

```html
<div class="coaches-search-panel" hx-disinherit="*">…</div>
<script>
(function () {
    const panel = document.currentScript.previousElementSibling;
    // tout le comportement est scoped à `panel`
})();
</script>
```

**Pourquoi :** évite les collisions si le même widget est présent plusieurs fois dans la page, et évite de polluer le namespace global.

### Règle 7 — Lifecycle des composants tiers (TomSelect, etc.)

Les composants JS tiers intégrés dans un widget sont wrappés dans un `x-data` Alpine avec `init()` et `destroy()` pour un cycle de vie propre lors des swaps HTMX.

```html
<div x-data="{
    init() { this._ts = new TomSelect(this.$refs.select, { ... }); },
    destroy() { this._ts?.destroy(); }
}">
    <select x-ref="select">…</select>
</div>
```

Ne jamais initialiser TomSelect (ou équivalent) dans un `<script>` nu sans lifecycle — le composant survivrait au remplacement du DOM et causerait des doublons.

---

## Pages complexes — pattern « page d'assemblage à widgets »

Pour toute page impliquant 3+ sections interactives indépendantes ou des données de plusieurs BCs, appliquer ce pattern (validé sur la refacto build-team).

### Architecture

```
Page hôte (build_team.rs + build-team.html)
│   Assemblage pur, quasi zéro JS.
│   Chaque section dynamique = un conteneur hx-get + hx-trigger.
│
├── Widget A (widgets/cart_widget.rs + templates/widgets/cart-widget.html)
│   Endpoint GET dédié, gère ses mutations, émet des événements DOM.
│
├── Widget B (widgets/player_table_widget.rs + ...)
│   Écoute les événements des autres widgets via hx-trigger="event from:body".
│
└── Domain service (use_cases/roster_service.rs)
    Transforme les DTOs du port en objets domaine.
    Les handlers appellent le service, jamais les DTOs directement.
```

### Principes

1. **La page hôte ne porte pas de logique** — pas de calcul de VMs, pas de JS d'orchestration, pas de macros Askama. Elle compose des `hx-get` + `hx-trigger`.
2. **Chaque widget est autonome** — endpoint GET (chargement) + endpoints POST (mutations), template isolé avec `hx-disinherit="*"`, JS scoped via Alpine `init()`/`destroy()`.
3. **Communication par événements DOM** — les widgets émettent via `HX-Trigger` header ou `htmx.trigger(document.body, ...)`, les consommateurs s'abonnent via `hx-trigger="eventName from:body"`.
4. **Données inter-BCs via ACL** — port dans `ports.rs`, adapter dans `src/infrastructure/<bc>/`, domain service dans `use_cases/` pour le mapping port → domaine.
5. **VMs purs domaine** : constructeurs `from_domain()` co-localisés. **VMs dépendant du port** : fonctions dans `builders.rs`.

### Quand appliquer

- Page avec 3+ sections interactives indépendantes
- Page qui combine des données de plusieurs BCs
- Page avec beaucoup de JS orchestrant des échanges HTMX (signe qu'il faut découper)

### Quand NE PAS appliquer

- Page simple avec un formulaire et une réponse (CRUD classique)
- Page statique avec un seul fragment HTMX

---

## Conventions templates (Askama + HTMX)

- Un template de **page complète** pour le premier chargement
- Des templates de **fragments** pour les réponses HTMX (swap partiel)
- Les structs de template ne portent que des **view models** — pas d'entités domaine

### Construction des view models — règle obligatoire

Les view models qui se construisent **uniquement à partir d'objets domaine** doivent exposer un constructeur `from_domain()` (ou `all_from_domain()` pour les collections) directement sur la struct VM. La logique de projection vit avec le type, pas dans un fichier builder séparé.

```rust
// CORRECT — constructeur co-localisé avec le VM
let cart = CartVm::from_domain(&roster_team);
let staff_rows = StaffRowVm::all_from_domain(&roster_team);
let reroll = RerollVm::from_domain(&roster_team);
```

Les view models qui dépendent de **DTOs de port** (données inter-BC) en plus du domaine restent construits par des fonctions dans `builders.rs`, car le fichier `view_models.rs` ne doit pas importer les types du port.

```rust
// CORRECT — builder séparé car dépend du port
let rows = build_hired_rows(&roster_team, &roster_def);
let positions = build_player_positions(&roster_def);
```

### Un view model transpose, il ne dérive pas — règle obligatoire

Toute valeur qui répond à une **question du domaine** vient d'une méthode du
domaine. Jamais d'un recalcul à partir de primitives dans la couche de
présentation, jamais d'une lecture de projection choisie parce qu'elle était
plus à portée.

Le test pour trancher : **si cette valeur s'avérait fausse, corrigerait-on la
vue ou le domaine ?** Si c'est le domaine, elle n'a rien à faire dans le VM. Et
si le domaine ne sait pas encore la dire, on l'y ajoute — on ne la déduit pas à
l'affichage.

```rust
// INTERDIT — la vue recompte ce que le domaine sait compter
let joueurs = squad_port.find_squad(&team_id).await.len();

// OBLIGATOIRE — le prédicat vient du domaine
let joueurs = membres.iter().filter(|m| m.presence.alignable()).count();
```

**Quatre défauts en trois cartes, tous de cette forme.** Le calcul était juste,
l'écran mentait :

| | ce que la vue faisait |
|---|---|
| 492 | affichait `players_proj.spp`, le cumul des gains, là où `spp_remaining()` donne le solde |
| 494 | inventait un jet de dé que le contrôleur ne lui avait pas passé |
| 495 | comptait `find_squad(...).len()` là où la règle des journaliers ne retient que les alignables |

Les trois se lisent pareil : **une donnée juste dans le domaine, fausse à
l'écran**. Aucun test unitaire de domaine ne les voyait — ils passaient tous.

Corollaire : **un gabarit n'invente aucune valeur.** Pas de `value="2"` en dur,
pas de repli silencieux sur une constante. Ce qu'il n'a pas reçu, il ne l'affiche
pas — un tiret, un champ vide, mais pas un chiffre plausible. Une valeur inventée
ne se distingue pas d'une valeur saisie, et c'est ce qui rend le défaut invisible
à la relecture.

### Le formatage rend la valeur, il ne la reconstruit pas — règle obligatoire

Découper un nombre pour le recomposer est une occasion de le perdre.

```js
// INTERDIT — recompose depuis les milliers et les centaines
const maj = Math.floor(v / 1000);
const min = Math.floor((v % 1000) / 100);
return min > 0 ? maj + ' ' + min + '00 kPo' : maj + ' 000 kPo';
```

Le chiffre des dizaines n'apparaît dans aucune des deux branches : **2075
s'affichait « 2 000 kPo »**, et la différence de valeur d'équipe avec. Perdu par
construction, pas par accident de calcul.

Cette fonction n'existait que dans un fichier, posée par le commit qui créait la
page. Les vingt-six autres affichages de kPo de l'application impriment la valeur
brute. **Une mise en forme inventée pour un seul écran est une divergence, pas un
raffinement** — et c'est le seul écran qui se trompait.

Le défaut est resté invisible parce qu'aucune équipe de la base locale
n'atteint 1 000 kPo, le seuil où il commence. Un test d'affichage lisant la
valeur à l'écran aurait passé ; celui qui l'attrape éprouve la fonction sur
**2075**, une valeur à dizaines non nulles.

### Ce que ces deux règles ne verrouillent pas

Aucun `grep` ne distingue « recomposer un nombre » de « le mettre en forme », ni
un recalcul fautif d'une projection légitime. Ce sont des **règles de revue**, au
même titre que les conventions de nommage de fichiers — appliquées au fil de
l'eau, sans axe de `check-arch` derrière elles.

Elles se rattrapent donc à la lecture, et à une question posée à chaque VM
écrit : *d'où vient ce chiffre, et qui le corrigerait s'il était faux ?*

### Selects — kreek-select obligatoire

Tout sélecteur dans l'application doit être un **`<kreek-select>`** (Web Component custom). Les `<select>` natifs et TomSelect sont **interdits** dans les templates finaux.

`kreek-select` gère automatiquement :
- Le chargement des données depuis une URL JSON (`url`)
- La recherche dans les options
- Le lifecycle (création / destruction sur swap HTMX via `connectedCallback` / `disconnectedCallback`)
- Les cascades entre selects (`listen` / `event`)
- Le rendu riche via `<template>` (`option-template` / `selected-template`)
- La sélection multiple avec badges (`multiple`)

```html
<!-- Exemple simple -->
<kreek-select name="fruit" url="/api/fruits" placeholder="Choisir un fruit…"></kreek-select>

<!-- Exemple cascade -->
<kreek-select name="color" url="/api/colors" event="colorSelected"></kreek-select>
<kreek-select name="fruit" url="/api/fruits" listen="colorSelected"
              listen-param="id" listen-query="color"></kreek-select>
```

- Le composant est défini dans `assets/static/js/kreek-select.js`, le CSS dans `assets/static/css/components/kreek-select.css`
- Page de test : `/kreek-select-tester`
- Les maquettes (`rawpages/`) peuvent utiliser des `<select>` natifs pour valider le rendu

### Interdiction des styles inline — règle obligatoire

Les attributs `style="..."` sont **totalement interdits** dans les templates HTML.

```html
<!-- INTERDIT -->
<div style="color: red; margin-top: 8px;">…</div>

<!-- OBLIGATOIRE — utiliser des classes CSS -->
<div class="text-error mt-2">…</div>
```

Tout besoin de style passe par des classes CSS définies dans les fichiers `.css` du projet (`assets/static/css/`).

### Réponses HTMX spéciales

```rust
// Redirect
Response::builder().header("HX-Redirect", "/dashboard").body(Body::empty()).unwrap()

// Refresh
Response::builder().header("HX-Refresh", "true").body(Body::empty()).unwrap()

// Trigger événement client
Response::builder().header("HX-Trigger", r#"{"showToast": "Sauvegardé"}"#).body(Body::empty()).unwrap()
```

---

## Responsivité

Approche **desktop-first** : les media queries utilisent `max-width`, on adapte le layout vers le bas à partir d'un rendu desktop de référence. Pas de framework CSS, pas de classes utilitaires génériques (`.hide-mobile`, `.col-*`, etc.) — chaque page gère sa propre responsivité avec ses propres classes.

### Breakpoint

Un seul breakpoint de référence pour toute l'application :

```css
@media (max-width: 768px) { ... }
```

Ce breakpoint marque la bascule desktop ↔ mobile/tablette. Le réutiliser systématiquement pour rester cohérent avec l'existant plutôt que d'introduire de nouvelles valeurs de coupure.

Exceptions ponctuelles déjà présentes dans le code (à ne pas généraliser sans raison) : `400px` (grille de chips joueurs), `640px`/`900px` (masquage progressif de colonnes de tableau).

### Chrome global — géré une seule fois, jamais par les pages

La bascule sidebar/menu desktop ↔ header/tabbar mobile est gérée intégralement par `app-layout.html` + `layout-app.css` + `app-menu.html` (markup desktop et mobile co-existent dans le même template, c'est le CSS qui bascule l'affichage via `@media`). **Aucune page de contenu ne doit réimplémenter cette logique** — elle n'a à se soucier que de son propre contenu interne.

Si un élément `position: fixed; bottom: 0` (cart, footer d'action) est utilisé dans une page, décaler son `bottom` sous 768px pour ne pas chevaucher la `.mobile-tabbar` globale (~64px de hauteur) :

```css
.mr-cart-footer { position: fixed; bottom: 0; left: 0; right: 0; }
@media (max-width: 768px) {
  .mr-container { padding-bottom: 180px; }
  .mr-cart-footer { bottom: 64px; }
}
```

### Layout de page

- Container central limité en largeur (`max-width: 900-980px; margin: 0 auto;`), en `flex-direction: column`, avec `gap` exprimé en tokens `var(--p0)` à `var(--p5)` (jamais de valeur `px` en dur pour l'espacement).
- Grilles régulières (chips, boutons d'action, cartes) : `display: grid; grid-template-columns: repeat(N, 1fr);`.
- Layouts asymétriques (article + sidebar, 2 colonnes) : `display: flex` avec des ratios `flex: N`, qui basculent en `flex-direction: column` sous 768px.

### Design tokens

Les tokens (couleurs, spacing `--p0..--p5`, typo `--text-*`, `--radius-*`) sont définis dans le `:root` de `assets/static/css/common.css`. Toujours réutiliser ces tokens plutôt que des valeurs en dur.

**Il n'existe pas de variable `--breakpoint-*`** — les breakpoints restent des valeurs `px` en dur dans chaque `@media`, à répéter telles quelles (`768px`) plutôt qu'inventées.

### Pattern mobile-first ponctuel (grilles à colonnes croissantes)

Pour une grille dont le nombre de colonnes doit croître avec l'espace disponible, le pattern `min-width` est accepté en exception au desktop-first global :

```css
.mr-player-chip-list { display: grid; grid-template-columns: repeat(2, 1fr); }
@media (min-width: 400px) { .mr-player-chip-list { grid-template-columns: repeat(3, 1fr); } }
@media (min-width: 768px) { .mr-player-chip-list { grid-template-columns: repeat(4, 1fr); } }
```

### Ce qui n'est pas utilisé — ne pas introduire sans discussion

- Pas de `clamp()` ni d'unités `vw`/`vh` fluides pour le texte — les ajustements de taille se font en dur par breakpoint.
- Pas de système de grille type Bootstrap (`.col-*`, `.row`).
- Pas de classes utilitaires responsive transverses (`.hide-mobile`, `.d-md-*`).

---

## Observabilité — règles obligatoires

En production, le journal est le seul organe de sens. L'épic E11 l'a construit ;
ces règles existent pour qu'il ne se reperde pas fichier par fichier. Les quatre
axes qui les tiennent sont bloquants — voir « Vérifications ».

### La règle qui prime sur les autres

**Une ligne de journal n'existe en production que si sa cible relève de
`kreek::` et que son niveau vaut au moins `info`.** Le filtre est
`kreek=<niveau>,sqlx=warn` ; une cible qui n'en relève pas n'est activée par
aucune directive, et la ligne n'est **pas** émise — sans que rien ne le
signale, ni à la compilation, ni aux tests, ni au démarrage.

Ce piège s'est présenté trois fois sous trois formes :

| Où | Ce qu'on croyait | Ce qui se passait |
|---|---|---|
| `TraceLayer` (carte 344) | le journal de requêtes existe | il émet sur `tower_http::trace`, hors filtre |
| `CatchPanicLayer` (carte 349) | un panic est journalisé | il émet sur `tower_http::catch_panic`, hors filtre |
| `#[instrument]` (carte 348) | le use case dit ce qu'on lui a demandé | un span n'émet **aucun** événement sans `FmtSpan` |

Il reparaîtra à chaque couche tierce branchée en comptant sur sa
journalisation intégrée : **une bibliothèque journalise sur son propre nom, et
notre filtre ne connaît que le nôtre.** Poser un gestionnaire maison qui émet
depuis un module `kreek::` règle le problème par construction.

### Les trois règles de couverture

**Tout use case async est instrumenté.** `#[tracing::instrument(skip_all,
fields(cmd = ?cmd))]` sur toute `pub async fn` de `use_cases/`. `skip_all` est
indispensable — sans lui l'attribut tente d'enregistrer les dépôts, qui
n'implémentent pas `Debug`. Un use case à identifiants nus nomme ses champs :
`fields(season_id = ?season_id)`.

Les fonctions async de `use_cases/` qui ne sont **pas** des use cases —
services d'hydratation, lectures — se déclarent sur place :

```rust
// arch:no-instrument — service d'hydratation : assemble une vue, sans intention métier
pub async fn hydrate(…)
```

Le motif est obligatoire. Une liste d'exceptions tenue dans `check-arch.sh`
aurait dérivé ; un marqueur adjacent à la fonction ne le peut pas.

**Toute émission d'événement passe par un helper.** `emettre()` pour un domain
event sur le bus interne, `publier()` pour un app event sur le bus applicatif.
Jamais de `.send(` direct.

Ce n'est pas une préférence de style : `to_enveloppe()` **engendre un nouvel
identifiant**, donc une ligne écrite à la main au-dessus d'un `send` reprendrait
celui de l'enveloppe reçue et produirait une trace qui a l'air correcte et ne
corrèle rien. Les helpers ne voient que l'enveloppe produite.

Un `.send(` qui n'est pas une émission d'événement — envoi d'e-mail, requête
HTTP — se déclare par `// arch:ok <motif>`, sur la ligne ou juste au-dessus.

**Les commandes ne journalisent pas de secrets.** Les champs sensibles d'une
commande sont des `Secret<T>` (`shared_kernel/identity/secret.rs`), dont le
`Debug` rend `[masqué]`. Le use case étant instrumenté avec `?cmd`, un champ
repassé en `String` mettrait des mots de passe dans `docker logs`.

### Ce que ça donne, et comment on s'en sert

```
grep rid=01M0AB   → la requête, ses use cases, les événements qu'elle a émis
grep <event_id>   → le publisher : cause=<reçu> event_id=<produit>
grep <event_id>   → toutes les réactions, tous BCs confondus
```

Le `rid` est repris dans l'en-tête `x-request-id` de la réponse : on part du
symptôme constaté par un coach, pas du code.

### Le réflexe à avoir

Devant une nouvelle couche, un nouveau bus, un nouveau point d'émission, la
question n'est pas « est-ce que ça journalise ? » mais **« sous quelle cible, à
quel niveau, et qu'est-ce qui le vérifie ? »**. Les trois fois où l'épic s'est
fait prendre, le code journalisait — dans le vide.

---

## Gestion des erreurs

`AppError` est l'enum central (`src/error.rs`) qui implémente `IntoResponse`.  
Il convertit automatiquement `sqlx::Error` et `DomainError` via `#[from]`.  
Les handlers HTMX reçoivent un fragment HTML d'erreur, pas du JSON.

---

## Configuration

Variables d'environnement au format `<SECTION>__<CLÉ>`, double underscore comme
séparateur — **sans préfixe**. `config::Environment::default()` n'en pose aucun :
un `APP__DATABASE__URL` se lirait `app.database.url`, chemin qui n'existe pas
dans `AppConfig`, et serait **ignoré en silence**. Cette section a documenté la
forme préfixée pendant des mois ; personne ne s'en est aperçu parce que les
fichiers `.env` et le `Makefile`, eux, ont toujours utilisé la bonne.

```bash
DATABASE__URL=postgres://user:pass@localhost/kreek_dev
SERVER__PORT=3210
LOG__LEVEL=info
```

Les valeurs par défaut vivent dans `config/default.toml`, surchargeables par
`config/<APP_ENV>.toml` puis par l'environnement. Cas particulier du niveau de
journalisation : **`RUST_LOG` supplante `LOG__LEVEL`** quand il est posé, pour
ouvrir un BC le temps d'une investigation sans toucher à la configuration
déployée.

---

## Base de données

- Migrations dans `migrations/` avec `sqlx migrate`
- Requêtes SQL dans des fichiers `.sql` dédiés sous `repositories/sql/`
- Utiliser `sqlx::query_as!` (macro vérifiée à la compilation) de préférence à `query_as`
- Tests d'intégration sur une vraie base de données — pas de mock sqlx

---

## Sessions

**Il n'y aura pas de Redis.** Ce fichier a longtemps annoncé une « phase 2 :
migration vers `RedisStore` ». La décision est prise, et c'est non : Postgres est
déjà là, déjà sauvegardé, déjà surveillé, et Redis ajouterait une infrastructure,
un point de panne et une sauvegarde de plus pour stocker quelques centaines de
sessions de ligue amateur — un volume où la différence de performance ne se
mesure pas. Le magasin persistant sera `tower-sessions-sqlx-store`.

Aujourd'hui : **`DashMapStore`, en mémoire** (`common/session_store.rs`), donc
volatile. Chaque redéploiement déconnecte tout le monde en même temps ; c'est le
défaut connu, et il reste à corriger.

Le cookie, lui, est configuré (carte 490) :

```rust
SessionManagerLayer::new(DashMapStore::new())
    .with_expiry(Expiry::OnInactivity(time::Duration::days(30)))
    .with_same_site(SameSite::Lax)
```

**Les deux lignes comptent, et leur absence ne se voit nulle part.**

Sans `with_expiry`, `tower-sessions` laisse `expiry: None` et le cookie n'a **ni
`Max-Age` ni `Expires`** : le navigateur le jette à sa fermeture, alors que le
serveur garde la session **deux semaines** (`DEFAULT_DURATION`). Le coach fermait
son navigateur le soir et se retrouvait déconnecté d'une session dont le serveur
se souvenait encore. `OnInactivity` fait glisser la fenêtre à chaque requête —
une date fixe déconnecterait tout le monde le même jour.

`SameSite::Strict`, l'autre défaut de la crate, **n'envoie pas le cookie** quand
on arrive depuis un lien extérieur : le coach qui suit un lien partagé atterrit
déconnecté sans avoir rien fait. `Lax` l'envoie sur une navigation de premier
niveau et le retient sur les requêtes inter-sites ; la garde qui compte contre le
CSRF reste le middleware maison, qui exige `HX-Request: true` sur toute mutation.

`src/web/tests/test_cookie_de_session.rs` lit l'en-tête **réellement émis** par
le routeur de production — pas la configuration censée le produire.

---

## Tests

- Tests unitaires dans un module `tests/` co-localisé avec le code
- Tests d'intégration repository : utilisent une vraie PgPool
- Fixtures SQL dans `tests/fixtures/*.sql`
- Ne pas mocker sqlx — les tests doivent frapper une vraie base

### Couverture obligatoire — règle fondamentale

Toute fonctionnalité livrée doit être couverte par :
1. **Un test unitaire** (`cargo test`) — logique domaine/use case, co-localisé avec le code.
2. **Un test end-to-end** (`tests/e2e/`, pytest + Playwright) — comportement réel dans un navigateur contre le serveur dev lancé.

Le test unitaire vérifie la logique ; le test E2E vérifie que le rendu HTML/HTMX/Alpine.js produit fonctionne réellement (le bug du widget coach-search et celui des pickers de tiers en phase 2 n'auraient été détectés par aucun test unitaire — uniquement par un test E2E piloté en navigateur).

Voir `tests/e2e/README.md` pour l'exécution (`make e2e`, nécessite le serveur dev lancé).

---

## Pièges frontend connus — Alpine.js + HTMX

### Alpine.js : chargement unique dans le layout de base

Alpine CDN est chargé **uniquement dans `app-layout.html`** (`<head>`, avec `defer`).  
Ne jamais l'inclure dans un `{% block content %}` de page individuelle.

**Pourquoi :** HTMX navigue sans rechargement complet — il ré-exécute les `<script>` trouvés dans le contenu swappé. Si deux pages chargent chacune le CDN Alpine, la navigation entre elles via HTMX crée une **deuxième instance Alpine** en mémoire. Les deux instances se disputent l'initialisation des composants `x-data`, notamment les fragments injectés dynamiquement via `htmx.ajax`. Symptôme typique : le composant fonctionne sur rechargement complet de la page (F5) mais pas lors de la navigation HTMX.

Les fonctions Alpine (`finalizePage`, `skillPicker`, etc.) restent dans des `<script>` inline des pages — HTMX les ré-exécute correctement à chaque navigation.

### Fragments HTMX : ne pas répéter l'`id` du conteneur cible

Quand un fragment est injecté via `htmx.ajax` avec `swap: 'innerHTML'`, l'élément racine du fragment **ne doit pas avoir le même `id` que son conteneur**.

```html
<!-- INTERDIT — id dupliqué dans le DOM après injection -->
<!-- Conteneur dans la page : -->
<div id="skill-picker-container" x-show="selectedPlayerId"></div>
<!-- Fragment retourné par le serveur : -->
<div id="skill-picker-container" x-data="skillPicker(...)">...</div>

<!-- CORRECT — le fragment est le contenu du conteneur, pas le conteneur lui-même -->
<div x-data="skillPicker(...)">...</div>
```

### La fenêtre où un élément est visible mais pas encore câblé

HTMX câble le contenu qu'il insère **quelques dizaines de millisecondes après
l'avoir rendu visible**. Pendant cette fenêtre, un bouton est peint, cliquable —
et inerte. Le clic s'y perd sans émettre de requête, sans erreur de console,
sans rien.

Mesuré sur l'étape de validation du magicien de compétition, trois fois de
suite, identique :

```
t=0ms    6 éléments htmx non câblés sur 31   ← dont « 🏆 Créer la compétition »
t=50ms   0
```

**Aucune attente habituelle ne voit cette fenêtre.** À `t=0` l'élément est
visible, son texte est le bon, plus aucune requête n'est en vol : tous les
signaux qu'on attend naturellement sont déjà verts. Attendre `.htmx-request` à
zéro ne suffit pas non plus — la classe est retirée *avant* que le contenu
inséré soit câblé.

En e2e, cliquer sur du contenu fraîchement injecté passe donc par
`tests/e2e/htmx_helpers.py` :

```python
from htmx_helpers import cliquer_quand_cable

cliquer_quand_cable(page, ".btn-toggle-spp")   # attend le câblage, puis clique
```

**Pas de `sleep`.** Une durée fixe n'a aucune marge sur une machine chargée — et
c'est exactement là que la suite échouait — tout en coûtant son délai aux
milliers d'appels où tout est déjà prêt. La condition rend la main dès que c'est
vrai : 7 à 20 ms mesurés.

Un humain n'atteint pas cette fenêtre : il lui faut deux à trois cents
millisecondes rien que pour réagir à ce qui vient de s'afficher. **C'est un
piège de test, pas un défaut produit** — mais il coûte cher à diagnostiquer,
parce que le symptôme, un clic qui ne produit strictement rien, ne ressemble
pas à un problème d'attente. Il a valu deux diagnostics faux — l'`id` dupliqué,
puis « htmx est aléatoire » — et une correction qui stabilisait le test par
accident, en ajoutant deux allers-retours vers le navigateur.

---

## Kanban — cycle de vie des cartes

```
to_be_refined → ready_to_be_done → done
                                  → cancelled
```

| Dossier | Contenu |
|---|---|
| `to_be_refined/` | Cartes avec questions ouvertes ou design incomplet |
| `ready_to_be_done/` | Cartes prêtes à implémenter, design validé |
| `done/` | Cartes implémentées, commitées et pushées |
| `cancelled/` | Cartes abandonnées (remplacées par un découpage, devenues obsolètes, scope abandonné) |

**Règle** : une carte est déplacée dans `done/` dans le **même commit** que le code qui la termine, ou dans le commit immédiatement suivant. Ne jamais laisser une carte en `ready_to_be_done/` après que son code a été pushé.

**Numérotation** : un numéro par carte, jamais réutilisé. Le prochain numéro
libre se lit ainsi :

```bash
find kanban -name "*.md" | sed 's|.*/||' | grep -oE "^[0-9]+" | sort -n | tail -1
```

Trois cartes ont porté le numéro 50 pendant deux mois et demi — créées le même
jour, sans que rien ne le signale. Un doublon ne se voit qu'en le cherchant :

```bash
find kanban -name "*.md" | sed 's|.*/||' | grep -oE "^[0-9]+" | sort -n | uniq -d
```

### Épics — la vue de haut niveau

`kanban/epics/` regroupe les cartes en **grandes fonctions**. L'épic dit
*pourquoi* et *quand c'est fini* ; les cartes disent *comment*. Sans elles, le
backlog n'est qu'une liste plate où l'on ne voit plus les chantiers.

```
to_be_refined → ready_to_be_done → en_cours → done
```

Mêmes noms de dossiers que les cartes, plus `en_cours/` — une épic est
commencée bien avant d'être close, et cet état-là n'existe pas pour une carte.

| Règle | |
|---|---|
| Nommage | `E<NN>-<slug>.md`, le préfixe `E` évitant toute collision avec la numérotation des cartes |
| Appartenance | une carte est dans **une seule** épic, ou dans aucune |
| État | l'épic suit **la plus en amont** de ses cartes : neuf prêtes et une à raffiner ⇒ l'épic reste en `to_be_refined/` |
| Sections | *La fonction* · *État* · *Les cartes* · *Ce qui commande l'ordre* · *Ce que l'épic ne couvre pas* · *Terminé quand* |

**« Terminé quand » est un critère observable**, jamais « toutes les cartes sont
dans `done/` ». Une épic close se constate à l'écran ou à la mesure — sinon
c'est la liste des cartes qui se relit elle-même, et l'épic n'apprend rien.

Pas d'épic « Divers » : les cartes sans épic sont **listées dans
`kanban/epics/README.md`**. Un sac simule une vue de haut niveau au lieu d'en
donner une.