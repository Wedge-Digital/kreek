EXEC_PROFILE ?= dev
DATABASE_URL   = $(shell grep -E '^DATABASE__URL=' .env.$(EXEC_PROFILE) | cut -d= -f2-)

.PHONY: dev test migrate db-prepare db-reset help

help:
	@echo "Targets disponibles :"
	@echo "  dev         Lance le serveur en mode watch (cargo-watch)"
	@echo "  test        Lance les tests (utilise .env.test)"
	@echo "  migrate              Applique les migrations SQLx"
	@echo "  new_migration        Crée une migration  (ex: make new_migration desc=create_teams)"
	@echo "  db-prepare           Régénère le cache sqlx (cargo sqlx prepare)"
	@echo "  db-reset             Remet la base à zéro (sqlx database reset)"
	@echo ""
	@echo "Variable : EXEC_PROFILE (défaut : dev)"

dev:
	cargo watch -x run -w src -w assets/templates -w assets/static/css

test:
	DATABASE_URL=$(shell grep -E '^DATABASE_URL=' .env.test | cut -d= -f2-) cargo test

migrate:
	DATABASE_URL=$(DATABASE_URL) sqlx migrate run

new_migration:
	@test -n "$(desc)" || (echo "Usage : make new_migration desc=<description>"; exit 1)
	DATABASE_URL=$(DATABASE_URL) sqlx migrate add $(desc)

db-prepare:
	DATABASE_URL=$(DATABASE_URL) cargo sqlx prepare

db-reset:
	DATABASE_URL=$(DATABASE_URL) sqlx database reset