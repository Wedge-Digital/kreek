#!/usr/bin/env bash
# changed_bcs.sh — résolution mécanique fichiers modifiés -> bounded contexts.
#
# Usage : scripts/impact/changed_bcs.sh [<ref>|<intervalle>]
#   sans argument : working tree + index vs HEAD, fichiers non suivis inclus
#   avec ref      : diff de la branche courante vs <ref> (ex: main), + working tree
#   avec A..B     : ce seul intervalle, sans working tree ni fichiers non suivis
#                   (sert au backtest : rejouer un commit passé à l'identique)
#
# Sortie : un jeton par ligne, dédupliqué.
#
#   <bc>                BC touché (nom de module sous src/app/)
#   @shared_kernel      noyau partagé du domaine (types, events agrégés, routes)
#   @core               plomberie transverse (main, state, web/, common/, cli/)
#   @migrations         migration SQL
#   @config             configuration
#   @build              Cargo.toml / lock / Makefile / askama.toml / CI
#   @assets             front global (CSS, JS, templates de base)
#   @templates:<bc>     template Askama d'un BC
#   @contract:<bc>      surface de contrat d'un BC : ports, événements domaine,
#                       publisher, listeners — ce par quoi les autres BCs le
#                       voient. Émis EN PLUS du jeton <bc>.
#   @e2e_harness        conftest / helpers partagés de la suite e2e
#   @test:<nom>         fichier de test e2e modifié (nom sans .py)
#   @unknown:<path>     fichier non résolu — trou de couverture du script
#
# Le script ne décide RIEN : il résout des chemins en jetons. L'interprétation
# (amplification, run-all) appartient au skill test-impact.
set -euo pipefail

REF="${1:-}"

collect_files() {
  # Intervalle explicite : on rejoue exactement ce diff-là, sans y mêler
  # l'état courant du working tree.
  if [[ "$REF" == *".."* ]]; then
    git diff --name-only "$REF"
    return
  fi
  if [[ -n "$REF" ]]; then
    # Trois points : ce que la branche a apporté depuis la divergence, sans
    # rejouer ce que `ref` a avancé de son côté.
    git diff --name-only "$REF...HEAD"
  fi
  # `git diff HEAD` couvre indexé + non indexé.
  git diff --name-only HEAD
  # Un fichier neuf pas encore `git add` est invisible de tout diff : sans ça,
  # un nouveau contrôleur ne déclencherait aucun test.
  git ls-files --others --exclude-standard
}

collect_files | sort -u | while IFS= read -r f; do
  [[ -z "$f" ]] && continue
  case "$f" in

    # ── Neutre : aucun impact sur le comportement testé ──────────────────
    docs/*|blog/*|kanban/*|memory/*|scripts/*|.claude/*|*.md|LICENSE)
        ;;
    assets/rawpages/*)          ;;   # maquettes, non servies par l'app
    tests/fixtures/*)           ;;   # jeux de données des tests unitaires Rust
    tests/impact-map.toml)      ;;   # la carte elle-même

    # ── Suite e2e ────────────────────────────────────────────────────────
    tests/e2e/test_*.py)
        name=$(basename "$f" .py)
        echo "@test:$name" ;;
    tests/e2e/*)
        # conftest, helpers partagés, pyproject, uv.lock : toute la suite en dépend.
        echo "@e2e_harness" ;;

    # ── Build & environnement ────────────────────────────────────────────
    Cargo.toml|Cargo.lock|Makefile|askama.toml|rust-toolchain*|.github/*)
        echo "@build" ;;

    # ── Configuration ────────────────────────────────────────────────────
    config/*|.env|.env.*|src/config.rs)
        echo "@config" ;;

    # ── Schéma ───────────────────────────────────────────────────────────
    migrations/*)
        echo "@migrations" ;;

    # ── Noyau partagé du domaine ─────────────────────────────────────────
    src/app/shared_kernel/*|src/app/all_domain_events.rs|src/app/routes.rs|src/app/mod.rs)
        echo "@shared_kernel" ;;

    # ── Plomberie transverse ─────────────────────────────────────────────
    src/main.rs|src/state.rs|src/web/*|src/common/*|src/cli/*|src/infrastructure/mod.rs)
        echo "@core" ;;

    # ── Templates Askama d'un BC ─────────────────────────────────────────
    src/app/*/io/web/templates/*)
        echo "@templates:$(echo "$f" | cut -d/ -f3)" ;;

    # ── Surface de contrat d'un BC ───────────────────────────────────────
    # Ports (ACL entrant), définitions d'événements et publisher (ACL sortant),
    # listeners (ce qu'il consomme des autres). Un changement ici peut casser
    # un BC voisin sans qu'aucun test du BC modifié ne s'en aperçoive : c'est
    # le seul cas où l'amplification par [deps] est justifiée.
    src/app/*/ports.rs|src/app/*/domain_event.rs|src/app/*/domain/domain_event.rs|\
    src/app/*/domain/events.rs|src/app/*/domain/events/*|src/app/*/io/app_events/*)
        bc=$(echo "$f" | cut -d/ -f3)
        echo "$bc"
        echo "@contract:$bc" ;;

    # ── Code d'un BC ─────────────────────────────────────────────────────
    src/app/*/*)
        echo "$(echo "$f" | cut -d/ -f3)" ;;

    # Les adapters inter-BC vivent hors du BC mais lui appartiennent :
    # `infrastructure/<bc>/` implémente les ports du BC *consommateur*.
    src/infrastructure/*/*)
        echo "$(echo "$f" | cut -d/ -f3)" ;;

    # ── Front global ─────────────────────────────────────────────────────
    # CSS et JS ne se rattachent à aucun BC de façon fiable, et un sélecteur
    # masqué casse un test e2e aussi sûrement qu'un bug serveur.
    assets/static/*|assets/templates/*)
        echo "@assets" ;;
    # Jeu de règles servi à toute l'application (rosters, inducements, …).
    assets/references*|assets/*.json)
        echo "@assets" ;;

    *)  echo "@unknown:$f" ;;
  esac
done | sort -u
