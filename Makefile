EXEC_PROFILE ?= dev

# Retire les guillemets encadrants d'une valeur lue dans un .env. Ils y sont
# nécessaires dès que le mot de passe contient &, ! ou ^ — sans eux, le
# `set -a; source` des scripts d'import casse la ligne et laisse la variable
# vide — mais ils n'ont rien à faire dans l'URL passée à sqlx.
unquote = sed -e "s/^['\"]//" -e "s/['\"]$$//"

# URL de la base par défaut (profil dev/legacy) : variable d'environnement
# DATABASE_URL si elle est fournie (CI, convention sqlx), sinon .env.<profile>.
# Le fallback fichier est silencieux si le .env est absent (2>/dev/null).
DATABASE_URL := $(if $(DATABASE_URL),$(DATABASE_URL),$(shell grep -E '^DATABASE__URL=' .env.$(EXEC_PROFILE) 2>/dev/null | cut -d= -f2- | $(unquote)))

# URL de la base de test : variable d'environnement DATABASE_URL_TEST si fournie
# (CI), sinon .env.test. Variable dédiée (pas DATABASE_URL) pour éviter qu'un
# `export DATABASE_URL=…dev…` local ne fasse cibler la base dev par `make test`.
TEST_DB_URL := $(if $(DATABASE_URL_TEST),$(DATABASE_URL_TEST),$(shell grep -E '^DATABASE__URL=' .env.test 2>/dev/null | cut -d= -f2- | $(unquote)))

# Profil de la base de démo, cible de `init_demo_db`.
DEMO_PROFILE  := demo
DEMO_ENV_FILE := .env.$(DEMO_PROFILE)

.PHONY: dev dev-demo test e2e test-impacted all_tests audit migrate migration prepare_db reset_db reset_test_db init_db \
        init_demo_db seed_accounts seed_e2e lint check-arch coverage analyze help

# ── Aide ──────────────────────────────────────────────────────────────────────
help:
	@echo ""
	@echo "  Développement"
	@echo "  ─────────────────────────────────────────────────────"
	@echo "  dev           Lance le serveur en mode watch"
	@echo "  dev-demo      Idem, mais servant le jeu de démo (assets/references.example) — requis par e2e"
	@echo "  test          Lance les tests (utilise .env.test)"
	@echo "  e2e           Lance les tests E2E Playwright (nécessite \`make dev-demo\` lancé)"
	@echo "  test-impacted Idem, mais uniquement les e2e impactés par le diff courant"
	@echo "  all_tests     test + e2e — garde-fou obligatoire avant tout commit (cf. CLAUDE.md)"
	@echo "  migrate       Échappatoire manuelle (le binaire applique déjà les migrations au boot)"
	@echo "  migration     Crée une migration (ex: make migration desc=create_teams)"
	@echo "  prepare_db    Régénère le cache sqlx (cargo sqlx prepare)"
	@echo "  reset_db      Remet la base à zéro (sqlx database reset)"
	@echo "  reset_test_db Remet la base de test à zéro (.env.test)"
	@echo "  init_db       reset_db + import des données legacy + seed comptes dev (WITH_SEED=1 pour aussi affecter les coachs aux spaces)"
	@echo "  init_demo_db  Idem sur la base de démo (.env.demo) — DROP DATABASE, double confirmation exigée"
	@echo "  seed_accounts Seed les comptes dev (scripts/seed_accounts.json)"
	@echo "  seed_e2e      Seed synthétique requis par la suite e2e (space + 12 coachs, idempotent)"
	@echo ""
	@echo "  Qualité & architecture"
	@echo "  ─────────────────────────────────────────────────────"
	@echo "  lint          Formatage (fmt) + linting (clippy)"
	@echo "  check-arch    Vérifications architecturales (axes 2–6)"
	@echo "  coverage      Couverture de tests (nécessite cargo-llvm-cov)"
	@echo "  analyze       lint + check-arch (pipeline CI complet)"
	@echo ""
	@echo "  Variable : EXEC_PROFILE (défaut : dev)"
	@echo ""

# ── Développement ─────────────────────────────────────────────────────────────
# `-w Cargo.toml` : sans lui, un changement de dépendance laissait le serveur
# tourner sur l'ancien binaire, **en silence** — découvert carte 273, où une
# vérification e2e attendait une reconstruction qui ne venait jamais.
#
# `Cargo.lock` n'est pas surveillé : un `cargo update -p <crate>` seul échappe
# donc encore au rechargement. Cas plus rare, et l'ajouter fait courir le risque
# qu'un `cargo run` qui retouche le verrou relance la boucle indéfiniment.
dev:
	cargo watch -x run -w src -w Cargo.toml -w assets/templates -w assets/static/css

# Force le jeu de démonstration versionné, quelle que soit la configuration
# locale. Utile quand `.env.dev` surcharge REFERENCES__DIR vers un jeu de
# règles réel : la suite e2e attend les rosters de `assets/references.example`
# (Granitiers, Zéphyriens, Lanterniers). Sans surcharge locale, `make dev`
# sert déjà ce jeu — c'est le défaut de config/default.toml.
dev-demo:
	REFERENCES__DIR=assets/references.example cargo watch -x run -w src -w Cargo.toml -w assets/templates -w assets/static/css

test: reset_test_db
	DATABASE_URL="$(TEST_DB_URL)" cargo test

e2e:
	cd tests/e2e && uv run pytest -v

# Filtre LOCAL de productivité : n'exécute que les tests e2e susceptibles
# d'être cassés par le diff courant (cf. .claude/skills/test-impact/SKILL.md).
# La CI exécute toujours la suite complète — ne jamais la restreindre à ça.
# Code 10 = une règle « run all » s'est déclenchée, on bascule sur make e2e.
# Code 11 = des BCs touchés n'ont aucune couverture e2e : on le dit, on ne
#           fait pas passer une sélection vide pour un succès.
test-impacted:
	@tests=$$(./scripts/impact/changed_bcs.sh $(REF) | ./scripts/impact/select_tests.py); \
	rc=$$?; \
	if [ $$rc -eq 10 ]; then \
	    echo ""; echo "  → suite complète"; echo ""; \
	    $(MAKE) --no-print-directory e2e; \
	elif [ $$rc -eq 11 ]; then \
	    exit 1; \
	elif [ -n "$$tests" ]; then \
	    echo ""; \
	    cd tests/e2e && uv run pytest -v $$tests; \
	fi

# Garde-fou avant commit (cf. CLAUDE.md, règle de collaboration obligatoire) :
# make ne continue à e2e que si test a réussi (prérequis Make standard).
all_tests: test e2e
	@echo ""
	@echo "  ✓ test + e2e : tout est vert"
	@echo ""

# Échappatoire manuelle : le binaire embarque les migrations et les applique
# automatiquement au démarrage (cf. run_migrations() dans main.rs). Cette
# target reste utile pour du troubleshooting ponctuel (appliquer une
# migration sans redémarrer le service).
migrate:
	DATABASE_URL="$(DATABASE_URL)" sqlx migrate run

migration:
	@test -n "$(desc)" || (echo "Usage : make migration desc=<description>"; exit 1)
	DATABASE_URL="$(DATABASE_URL)" sqlx migrate add $(desc)

prepare_db:
	DATABASE_URL="$(DATABASE_URL)" cargo sqlx prepare

reset_db:
	DATABASE_URL="$(DATABASE_URL)" sqlx database reset -y -f

reset_test_db:
	DATABASE_URL="$(TEST_DB_URL)" sqlx database reset -y -f

seed_accounts:
	DATABASE_URL="$(DATABASE_URL)" cargo run -- seed-accounts

# Seed synthétique de la suite e2e : un space, DevCoach (legacy_id=1, connecté
# par BYPASS_AUTH) et onze autres coachs. Idempotent — rejouable sans risque.
seed_e2e:
	DATABASE_URL="$(DATABASE_URL)" cargo run -- seed-e2e

init_db: reset_db
	@echo ""
	@echo "  Import des données legacy…"
	@./scripts/import_all.sh
	@echo ""
	@echo "  Seed des comptes dev…"
	@DATABASE_URL="$(DATABASE_URL)" cargo run -- seed-accounts
ifeq ($(WITH_SEED),1)
	@echo ""
	@echo "  Affectation des coachs aux spaces…"
	@./scripts/seed_space_members.sh
endif
	@echo ""
	@echo "  ✓ Base initialisée"
	@echo ""

# Même chose, mais sur la base de démo (.env.demo) — donc potentiellement
# distante et partagée. `sqlx database reset` y fait un DROP DATABASE : la
# double confirmation est là parce qu'une faute de profil est irrattrapable.
#
# DATABASE_URL est relu depuis .env.demo et passé explicitement au sous-make :
# sans ça, un `export DATABASE_URL=…dev…` dans le shell appelant l'emporterait
# (cf. le `$(if $(DATABASE_URL),…)` en tête de fichier) et on réinitialiserait
# la base dev tout en important dans la démo.
init_demo_db:
	@[ -f $(DEMO_ENV_FILE) ] || { \
	    echo ""; \
	    echo "  Erreur : $(DEMO_ENV_FILE) introuvable."; \
	    echo "  Créez-le avec DATABASE__URL et DATABASE__HOST/PORT/USER/PWD/NAME"; \
	    echo "  (les deux représentations sont nécessaires : l'URL pour sqlx,"; \
	    echo "   les cinq variables pour les scripts d'import)."; \
	    echo ""; \
	    exit 1; \
	}
	@url=$$(grep -E '^DATABASE__URL='  $(DEMO_ENV_FILE) | cut -d= -f2- | $(unquote)); \
	 host=$$(grep -E '^DATABASE__HOST=' $(DEMO_ENV_FILE) | cut -d= -f2- | $(unquote)); \
	 name=$$(grep -E '^DATABASE__NAME=' $(DEMO_ENV_FILE) | cut -d= -f2- | $(unquote)); \
	 [ -n "$$url" ] && [ -n "$$host" ] && [ -n "$$name" ] || { \
	     echo "  Erreur : DATABASE__URL, DATABASE__HOST ou DATABASE__NAME manquant dans $(DEMO_ENV_FILE)."; exit 1; }; \
	 case "$$url" in \
	     *"$$host"*) ;; \
	     *) echo "  Erreur : DATABASE__URL ne pointe pas sur DATABASE__HOST ($$host) dans $(DEMO_ENV_FILE)."; \
	        echo "  Refus : sqlx et les scripts d'import viseraient deux bases différentes."; exit 1 ;; \
	 esac; \
	 case "$$url" in \
	     *"$$name"*) ;; \
	     *) echo "  Erreur : DATABASE__URL ne pointe pas sur DATABASE__NAME ($$name) dans $(DEMO_ENV_FILE)."; \
	        echo "  Refus : sqlx et les scripts d'import viseraient deux bases différentes."; exit 1 ;; \
	 esac; \
	 echo ""; \
	 echo "  \033[1m\033[31m/!\\  DESTRUCTION COMPLÈTE DE LA BASE DE DÉMO\033[0m"; \
	 echo ""; \
	 echo "     Profil : $(DEMO_PROFILE)   ($(DEMO_ENV_FILE))"; \
	 echo "     Hôte   : $$host"; \
	 echo "     Base   : $$name"; \
	 echo ""; \
	 echo "  sqlx va faire un DROP DATABASE puis tout réimporter."; \
	 echo "  Les comptes, espaces et articles existants seront perdus."; \
	 echo ""; \
	 printf "  Confirmation 1/2 — tapez \033[1moui\033[0m pour continuer : "; \
	 read -r answer; \
	 [ "$$answer" = "oui" ] || { echo "  Annulé."; exit 1; }; \
	 printf "  Confirmation 2/2 — retapez le nom de la base (\033[1m%s\033[0m) : " "$$name"; \
	 read -r confirm; \
	 [ "$$confirm" = "$$name" ] || { echo "  Annulé — le nom saisi ne correspond pas."; exit 1; }; \
	 echo ""; \
	 $(MAKE) init_db EXEC_PROFILE=$(DEMO_PROFILE) DATABASE_URL="$$url"

# ── Qualité Rust standard (axe 1) ────────────────────────────────────────────
lint:
	@echo ""
	@echo "\033[1m\033[34m┌─ Axe 1 · Qualité Rust standard\033[0m"
	@echo ""
	@echo "  \033[1mFormatage...\033[0m"
	@cargo fmt --check
	@echo "  \033[32m✓ PASS\033[0m  cargo fmt"
	@echo ""
	@echo "  \033[1mLinting (correctness + unused imports)...\033[0m"
	@if cargo clippy --tests -- \
		-D clippy::correctness \
		-D unused-imports \
		-D unreachable-patterns \
		-D irrefutable-let-patterns \
		2>&1 | tee /tmp/kreek-clippy.log | grep -qE "^error"; then \
		echo "  \033[31m✗ FAIL\033[0m  cargo clippy"; \
		grep "^error" /tmp/kreek-clippy.log; \
		exit 1; \
	fi
	@echo "  \033[32m✓ PASS\033[0m  cargo clippy"
	@echo ""
	@WARN_COUNT=$$(cargo clippy 2>&1 | grep -c "^warning:" || true); \
		echo "  \033[33m⚠\033[0m  $$WARN_COUNT warning(s) de style non-bloquants — \`cargo clippy\` pour le détail"
	@echo ""

# ── Vérifications architecturales (axes 2–6) ──────────────────────────────────
# Audit des dépendances — cible dédiée, branchée sur le job `audit` de la CI.
#
# Sortie de `make lint` volontairement : un avis publié cette nuit par RustSec
# ferait échouer `fmt` et `clippy`, dont il ne dit rien, sous un titre
# « Qualité » qui induirait en erreur. Un échec d'audit doit se lire comme tel.
#
# `--deny warnings` n'est pas du zèle : les avis `unmaintained` et `unsound` ne
# sont pas des vulnérabilités et ne bloqueraient pas sans lui. Or au moment
# d'écrire cette cible, le seul avis portant sur une crate **réellement
# compilée** était de ce type (`atty`), quand la seule « vulnérabilité »
# signalée portait sur une crate absente du binaire. Sans `--deny warnings`,
# l'audit criait sur ce qui ne nous concernait pas et se taisait sur le reste.
#
# En local, l'absence du binaire est tolérée : on n'impose pas une installation
# à qui lance `make lint` pour vérifier un formatage. En CI elle échoue — sans
# quoi il suffirait que le binaire disparaisse du runner pour retrouver le vert
# silencieux que cette cible existe pour supprimer (cf. carte 272).
#
# Pour débloquer un échec : `cargo tree -i <crate> -e all --target all`. Rien
# n'est imprimé → entrée de verrou jamais compilée, à ignorer dans
# `.cargo/audit.toml` avec son motif. Un chemin est imprimé → exposition réelle,
# à corriger par une montée de version ou un remplacement.
audit:
	@echo ""
	@echo "\033[1m\033[34m┌─ Audit des dépendances\033[0m"
	@echo ""
	@if command -v cargo-audit >/dev/null 2>&1; then \
		cargo audit --deny warnings && echo "  \033[32m✓ PASS\033[0m  cargo audit"; \
	elif [ -n "$$CI" ]; then \
		echo "  \033[31m✗ FAIL\033[0m  cargo-audit absent du runner — l'étape ne peut pas être sautée en CI"; \
		exit 1; \
	else \
		echo "  \033[33m⚠ SKIP\033[0m  cargo-audit non installé (cargo install cargo-audit --locked)"; \
	fi
	@echo ""

check-arch:
	@./scripts/check-arch.sh all

# ── Couverture de tests (axe 7) ───────────────────────────────────────────────
coverage:
	@if ! command -v cargo-llvm-cov >/dev/null 2>&1; then \
		echo ""; \
		echo "  \033[33m⚠\033[0m  cargo-llvm-cov non installé."; \
		echo "     Installer avec : cargo install cargo-llvm-cov"; \
		echo "     Puis relancer  : make coverage"; \
		echo ""; \
		exit 1; \
	fi
	@echo ""
	@echo "\033[1m\033[34m┌─ Axe 7 · Couverture de tests\033[0m"
	@echo ""
	DATABASE_URL="$(TEST_DB_URL)" \
		cargo llvm-cov \
		--ignore-filename-regex="(tests|io/web|io/repository|main\.rs)" \
		--summary-only
	@echo ""
	@echo "  Rapport HTML : cargo llvm-cov --html --open"
	@echo ""

# ── Pipeline complet ──────────────────────────────────────────────────────────
analyze: lint check-arch
	@echo ""
	@echo "\033[1m\033[32m  ✓ Pipeline d'analyse terminé\033[0m"
	@echo ""
