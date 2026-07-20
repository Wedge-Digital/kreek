EXEC_PROFILE ?= dev
DATABASE_URL   = $(shell grep -E '^DATABASE__URL=' .env.$(EXEC_PROFILE) | cut -d= -f2-)

.PHONY: dev test e2e all_tests migrate migration prepare_db reset_db reset_test_db init_db \
        seed_accounts lint check-arch coverage analyze help

# ── Aide ──────────────────────────────────────────────────────────────────────
help:
	@echo ""
	@echo "  Développement"
	@echo "  ─────────────────────────────────────────────────────"
	@echo "  dev           Lance le serveur en mode watch"
	@echo "  test          Lance les tests (utilise .env.test)"
	@echo "  e2e           Lance les tests E2E Playwright (nécessite le serveur dev lancé)"
	@echo "  all_tests     test + e2e — garde-fou obligatoire avant tout commit (cf. CLAUDE.md)"
	@echo "  migrate       Applique les migrations SQLx"
	@echo "  migration     Crée une migration (ex: make migration desc=create_teams)"
	@echo "  prepare_db    Régénère le cache sqlx (cargo sqlx prepare)"
	@echo "  reset_db      Remet la base à zéro (sqlx database reset)"
	@echo "  reset_test_db Remet la base de test à zéro (.env.test)"
	@echo "  init_db       reset_db + import des données legacy + seed comptes dev (WITH_SEED=1 pour aussi affecter les coachs aux spaces)"
	@echo "  seed_accounts Seed les comptes dev (scripts/seed_accounts.json)"
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
dev:
	cargo watch -x run -w src -w assets/templates -w assets/static/css

test: reset_test_db
	DATABASE_URL=$(shell grep -E '^DATABASE__URL=' .env.test | cut -d= -f2-) cargo test

e2e:
	cd tests/e2e && uv run pytest -v

# Garde-fou avant commit (cf. CLAUDE.md, règle de collaboration obligatoire) :
# make ne continue à e2e que si test a réussi (prérequis Make standard).
all_tests: test e2e
	@echo ""
	@echo "  ✓ test + e2e : tout est vert"
	@echo ""

migrate:
	DATABASE_URL=$(DATABASE_URL) sqlx migrate run

migration:
	@test -n "$(desc)" || (echo "Usage : make migration desc=<description>"; exit 1)
	DATABASE_URL=$(DATABASE_URL) sqlx migrate add $(desc)

prepare_db:
	DATABASE_URL=$(DATABASE_URL) cargo sqlx prepare

reset_db:
	DATABASE_URL=$(DATABASE_URL) sqlx database reset -y -f

reset_test_db:
	DATABASE_URL=$(shell grep -E '^DATABASE__URL=' .env.test | cut -d= -f2-) sqlx database reset -y -f

seed_accounts:
	DATABASE_URL=$(DATABASE_URL) cargo run -- seed-accounts

init_db: reset_db
	@echo ""
	@echo "  Import des données legacy…"
	@./scripts/import_all.sh
	@echo ""
	@echo "  Seed des comptes dev…"
	@DATABASE_URL=$(DATABASE_URL) cargo run -- seed-accounts
ifeq ($(WITH_SEED),1)
	@echo ""
	@echo "  Affectation des coachs aux spaces…"
	@./scripts/seed_space_members.sh
endif
	@echo ""
	@echo "  ✓ Base initialisée"
	@echo ""

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
	@if command -v cargo-audit >/dev/null 2>&1; then \
		echo "  \033[1mAudit des dépendances...\033[0m"; \
		cargo audit && echo "  \033[32m✓ PASS\033[0m  cargo audit"; \
	else \
		echo "  \033[33m⚠ SKIP\033[0m  cargo audit non installé (cargo install cargo-audit)"; \
	fi
	@echo ""

# ── Vérifications architecturales (axes 2–6) ──────────────────────────────────
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
	DATABASE_URL=$(shell grep -E '^DATABASE__URL=' .env.test | cut -d= -f2-) \
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
