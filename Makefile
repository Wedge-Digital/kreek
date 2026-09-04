EXEC_PROFILE ?= dev
SOURCE_PROFILE ?= remote.prod
TARGET_PROFILE ?= dev
YES ?= 0

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

# Profil de la base de démo, cible de `init_remote_demo_db`.
#
# Nommé `remote.demo` et non `demo` : c'est le seul profil dont la base vit
# ailleurs que sur la machine, et rien dans « demo » ne le disait. La confusion
# a coûté plusieurs allers-retours pendant la carte 307, où `make dev-demo` —
# qui ne choisit que le référentiel — a été pris pour ce profil-ci.
DEMO_PROFILE  := remote.demo
DEMO_ENV_FILE := .env.$(DEMO_PROFILE)

# Profil de la base de production, cible de `init_remote_prod_db`. Même
# convention de nommage que la démo — le préfixe `remote.` dit que la base vit
# ailleurs que sur la machine.
PROD_PROFILE  := remote.prod
PROD_ENV_FILE := .env.$(PROD_PROFILE)

.PHONY: dev dev-demo test e2e test-impacted all_tests audit migrate migration prepare_db reset_db reset_test_db init_db \
        load_data create_remote_db init_remote_data init_remote_db init_remote_demo_db init_remote_prod_db seed_accounts seed_e2e lint check-arch coverage analyze help

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
	@echo "  init_remote_demo_db  Idem sur la base de démo distante ($(DEMO_ENV_FILE)) — DROP+CREATE...OWNER via un accès admin séparé, double confirmation exigée"
	@echo "  init_remote_prod_db  Première mise en service de la production ($(PROD_ENV_FILE)) — DROP+CREATE...OWNER, refuse une base déjà peuplée, sans seed de comptes, triple confirmation"
	@echo "  import_prod_db  Recopie la production dans une base locale — détruit la cible, refuse toute cible distante (TARGET_PROFILE=test, YES=1)"
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
#
# Le niveau de journalisation n'est **pas** posé ici : il vient de `LOG__LEVEL`
# (`.env.dev`, `config/default.toml`), comme en production. Un `RUST_LOG` en
# dur dans cette cible aurait supplanté la configuration, et le seul chemin
# jamais exercé en local serait celui qui tourne en production. Pour ouvrir un
# BC le temps d'une investigation : `RUST_LOG=kreek::app::players=debug make dev`.
dev:
	cargo watch -x run -w src -w Cargo.toml -w assets/templates -w assets/static/css

# Force le jeu de démonstration versionné, quelle que soit la configuration
# locale. Utile quand `.env.dev` surcharge REFERENCES__DIR vers un jeu de
# règles réel : la suite e2e attend les rosters de `assets/references.example`
# (Granitiers, Zéphyriens, Lanterniers). Sans surcharge locale, `make dev`
# sert déjà ce jeu — c'est le défaut de config/default.toml.
dev-demo:
	REFERENCES__DIR=assets/references.example EMAIL__PROVIDER=console cargo watch -x run -w src -w Cargo.toml -w assets/templates -w assets/static/css

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
# Tout autre code non nul = le sélecteur n'a pas tourné. Il tombait auparavant
# dans le `elif` final, où une sélection vide ne déclenchait rien et la cible
# rendait zéro : `select_tests.py` a planté des jours sur un `import tomllib`
# sans qu'aucun test ne s'exécute, et sans que rien ne rougisse (carte 480).
test-impacted:
	@tests=$$(./scripts/impact/changed_bcs.sh $(REF) | ./scripts/impact/select_tests.py); \
	rc=$$?; \
	if [ $$rc -eq 10 ]; then \
	    echo ""; echo "  → suite complète"; echo ""; \
	    $(MAKE) --no-print-directory e2e; \
	elif [ $$rc -eq 11 ]; then \
	    exit 1; \
	elif [ $$rc -ne 0 ]; then \
	    echo ""; \
	    echo "  ✗ le sélecteur de tests n'a pas tourné (code $$rc) — voir ci-dessus"; \
	    echo ""; \
	    exit $$rc; \
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

# ── Garde-fou : une cible destructrice ne vise pas une base distante ──────────
#
# `reset_db` fait `sqlx database reset -y -f` : DROP + CREATE + migrate, sans
# la moindre confirmation. Il travaille sur `DATABASE_URL`, résolu depuis
# `.env.$(EXEC_PROFILE)` — donc `make reset_db EXEC_PROFILE=remote.demo`
# détruisait la base distante en silence, et un `export DATABASE_URL=…distant…`
# oublié dans le shell suffisait aussi.
#
# La garde refuse tout hôte qui n'est pas local. Elle se contourne
# explicitement, ce que fait `init_remote_demo_db` — lui vise une base distante
# par construction, et porte déjà ses propres vérifications et sa double
# confirmation.
#
# Elle protège quel que soit le nom du profil : c'est ce qui la rend plus utile
# que le renommage de `.env.remote.demo`, lequel n'informe qu'au moment où on
# écrit la commande, pas quand on la relance depuis l'historique.
define refuser_si_distant
	@url="$(1)"; \
	if [ "$$I_KNOW_THIS_IS_REMOTE" != "1" ]; then \
	    hote=$$(printf '%s' "$$url" | sed -E 's#^[^:]+://([^/]*@)?([^:/?]+).*#\2#'); \
	    case "$$hote" in \
	        localhost|127.0.0.1|::1|"") ;; \
	        *) echo ""; \
	           echo "  \033[1m\033[31m/!\\  Refus : cible destructrice sur un hôte distant\033[0m"; \
	           echo ""; \
	           echo "     Hôte   : $$hote"; \
	           echo "     Profil : $(EXEC_PROFILE)"; \
	           echo ""; \
	           echo "  Cette cible détruit la base sans confirmation. Si c'est"; \
	           echo "  délibéré : I_KNOW_THIS_IS_REMOTE=1 make <cible>"; \
	           echo ""; \
	           exit 1 ;; \
	    esac; \
	fi
endef

migrate:
	DATABASE_URL="$(DATABASE_URL)" sqlx migrate run

migration:
	@test -n "$(desc)" || (echo "Usage : make migration desc=<description>"; exit 1)
	DATABASE_URL="$(DATABASE_URL)" sqlx migrate add $(desc)

# `--all-targets` : `make lint` lance `cargo clippy --tests`, donc les requêtes
# écrites dans un module de test doivent être au cache elles aussi. Sans lui, la
# cible ne peut pas satisfaire le lint qu'elle sert — constaté carte 427.
prepare_db:
	DATABASE_URL="$(DATABASE_URL)" cargo sqlx prepare -- --all-targets

reset_db:
	$(call refuser_si_distant,$(DATABASE_URL))
	DATABASE_URL="$(DATABASE_URL)" sqlx database reset -y -f

# Recopie la production dans une base locale. Le script porte ses trois gardes
# et la question ; voir son en-tête pour ce qu'il détruit et ce qu'il ne touche
# jamais.
#
#   make import_prod_db                       # prod → dev
#   make import_prod_db TARGET_PROFILE=test   # prod → test
#   YES=1 make import_prod_db                 # sans la question
import_prod_db:
	@SOURCE_PROFILE="$(SOURCE_PROFILE)" TARGET_PROFILE="$(TARGET_PROFILE)" YES="$(YES)" \
	 bash scripts/import_prod_db.sh

reset_test_db:
	$(call refuser_si_distant,$(TEST_DB_URL))
	DATABASE_URL="$(TEST_DB_URL)" sqlx database reset -y -f

seed_accounts:
	DATABASE_URL="$(DATABASE_URL)" cargo run -- seed-accounts

# Seed synthétique de la suite e2e : un space, DevCoach (celui que BYPASS_AUTH
# connecte, repéré par son nom) et onze autres coachs. Idempotent — rejouable
# sans risque, et installable par-dessus les données legacy.
seed_e2e:
	DATABASE_URL="$(DATABASE_URL)" cargo run -- seed-e2e

init_db: reset_db
	@$(MAKE) --no-print-directory load_data

# Import legacy + seed comptes dev (+ affectation coachs aux spaces si
# WITH_SEED=1). Partagé par init_db (après reset_db) et init_remote_demo_db (après
# create_demo_db + migrate) — seule la création de la base diffère entre les
# deux, le chargement des données est identique.
load_data:
	@echo ""
	@echo "  Import des données legacy…"
	@./scripts/import_all.sh
ifneq ($(WITH_ACCOUNTS),0)
	@echo ""
	@echo "  Seed des comptes dev…"
	@DATABASE_URL="$(DATABASE_URL)" cargo run -- seed-accounts
endif
ifeq ($(WITH_SEED),1)
	@echo ""
	@echo "  Affectation des coachs aux spaces…"
	@./scripts/seed_space_members.sh
endif
	@echo ""
	@echo "  ✓ Base initialisée"
	@echo ""

# DROP + CREATE explicite avec OWNER — n'est appelé que par init_remote_demo_db.
# `sqlx database reset` (utilisé par reset_db/init_db) DROP+CREATE+migrate en
# une seule connexion : la base est alors possédée par qui l'exécute. Pour la
# démo, distante et partagée, DATABASE__USER doit rester un compte restreint,
# pas un compte avec droit CREATEDB. ADMIN_URL (droits admin, connecté sur une
# base de maintenance) fait donc le DROP + CREATE ... OWNER, et DB_OWNER
# devient propriétaire dès la création — aucun droit d'administration ne lui
# est nécessaire ensuite pour migrer ou importer.
#
# WITH (FORCE) (PG13+) coupe les connexions actives sans prévenir : accepté
# ici car la base de démo est jetable par construction (double confirmation
# déjà exigée par init_remote_demo_db) — ne pas généraliser ce comportement ailleurs.
# `WITH (FORCE)` coupe les connexions actives sans prévenir, pour les deux
# profils.
#
# Un temps réservé à la démo, jetable par construction. Étendu à la production
# parce qu'une **première** mise en service se fait serveur arrêté ou presque, et
# qu'un DROP qui échoue faute d'avoir fermé une session oubliée n'apprend rien —
# il oblige seulement à recommencer. La protection qui compte n'est pas là :
# c'est `REFUS_SI_PEUPLEE`, qui arrête la commande sur une base qui a déjà des
# comptes.
#
# Le comportement n'est pas paramétré : les deux appelants veulent le même, et
# une branche que personne n'emprunte rassure sans jamais être exercée.
create_remote_db:
	@psql "$(ADMIN_URL)" -v ON_ERROR_STOP=1 \
	    -c "DROP DATABASE IF EXISTS \"$(DB_NAME)\" WITH (FORCE);" \
	    -c "CREATE DATABASE \"$(DB_NAME)\" OWNER \"$(DB_OWNER)\" ENCODING 'UTF8';"

# Migrations + chargement des données sur la base de démo, une fois
# create_demo_db passé. Migre avec l'accès applicatif (DATABASE_URL) : il est
# déjà propriétaire de la base fraîchement créée, donc habilité à créer les
# objets des migrations sans droit admin.
init_remote_data: migrate
	@$(MAKE) --no-print-directory load_data WITH_ACCOUNTS=$(WITH_ACCOUNTS)

# Même chose que init_db, mais sur la base de démo — distante et partagée par
# construction, d'où le préfixe `remote.` de son profil. Contrairement à init_db, la base
# n'est pas recréée par le compte applicatif : create_demo_db sépare l'accès
# admin (DROP/CREATE ... OWNER) de l'accès applicatif restreint (migrations,
# import). La double confirmation reste là parce qu'une faute de profil est
# irrattrapable.
#
# DATABASE_URL est relu depuis $(DEMO_ENV_FILE) et passé explicitement aux
# sous-make :
# sans ça, un `export DATABASE_URL=…dev…` dans le shell appelant l'emporterait
# (cf. le `$(if $(DATABASE_URL),…)` en tête de fichier) et on réinitialiserait
# la base dev tout en important dans la démo.
# Cible générique des mises en service distantes. Ne s'appelle pas directement :
# `init_remote_demo_db` et `init_remote_prod_db` la paramètrent.
#
# `make -n init_remote_db` **n'est pas un essai à blanc**. Make exécute les
# lignes `$(MAKE)` même sous `-n`, donc les contrôles de cohérence et les
# interrogations de la base ont réellement lieu. Seule la partie destructrice
# reste protégée — par les confirmations, qui échouent faute d'entrée.
init_remote_db:
	@[ -f $(ENV_FILE) ] || { \
	    echo ""; \
	    echo "  Erreur : $(ENV_FILE) introuvable."; \
	    echo "  Créez-le avec DATABASE__URL, DATABASE__ADMIN_URL et"; \
	    echo "  DATABASE__HOST/PORT/USER/PWD/NAME"; \
	    echo "  (DATABASE__ADMIN_URL : accès admin distinct, connecté sur la"; \
	    echo "   base de maintenance 'postgres', droits CREATEDB — utilisé"; \
	    echo "   uniquement pour DROP/CREATE DATABASE. DATABASE__URL reste"; \
	    echo "   l'accès applicatif restreint, propriétaire de la base dès sa"; \
	    echo "   création. Les cinq variables DATABASE__* restent nécessaires"; \
	    echo "   pour les scripts d'import)."; \
	    echo ""; \
	    exit 1; \
	}
	@url=$$(grep -E '^DATABASE__URL='       $(ENV_FILE) | cut -d= -f2- | $(unquote)); \
	 admin_url=$$(grep -E '^DATABASE__ADMIN_URL=' $(ENV_FILE) | cut -d= -f2- | $(unquote)); \
	 host=$$(grep -E '^DATABASE__HOST=' $(ENV_FILE) | cut -d= -f2- | $(unquote)); \
	 user=$$(grep -E '^DATABASE__USER=' $(ENV_FILE) | cut -d= -f2- | $(unquote)); \
	 name=$$(grep -E '^DATABASE__NAME=' $(ENV_FILE) | cut -d= -f2- | $(unquote)); \
	 [ -n "$$url" ] && [ -n "$$admin_url" ] && [ -n "$$host" ] && [ -n "$$user" ] && [ -n "$$name" ] || { \
	     echo "  Erreur : DATABASE__URL, DATABASE__ADMIN_URL, DATABASE__HOST, DATABASE__USER ou DATABASE__NAME manquant dans $(ENV_FILE)."; exit 1; }; \
	 case "$$url" in \
	     *"$$host"*) ;; \
	     *) echo "  Erreur : DATABASE__URL ne pointe pas sur DATABASE__HOST ($$host) dans $(ENV_FILE)."; \
	        echo "  Refus : sqlx et les scripts d'import viseraient deux bases différentes."; exit 1 ;; \
	 esac; \
	 case "$$url" in \
	     *"$$name"*) ;; \
	     *) echo "  Erreur : DATABASE__URL ne pointe pas sur DATABASE__NAME ($$name) dans $(ENV_FILE)."; \
	        echo "  Refus : sqlx et les scripts d'import viseraient deux bases différentes."; exit 1 ;; \
	 esac; \
	 case "$$url" in \
	     *"$$user"*) ;; \
	     *) echo "  Erreur : DATABASE__URL n'utilise pas DATABASE__USER ($$user) dans $(ENV_FILE)."; \
	        echo "  Refus : la base serait créée avec ce compte comme OWNER, mais migrée/importée avec un autre."; exit 1 ;; \
	 esac; \
	 case "$$admin_url" in \
	     *"$$host"*) ;; \
	     *) echo "  Erreur : DATABASE__ADMIN_URL ne pointe pas sur DATABASE__HOST ($$host) dans $(ENV_FILE)."; exit 1 ;; \
	 esac; \
	 admin_db=$${admin_url##*/}; admin_db=$${admin_db%%\?*}; \
	 [ "$$admin_db" != "$$name" ] || { \
	     echo "  Erreur : DATABASE__ADMIN_URL pointe sur DATABASE__NAME ($$name)."; \
	     echo "  Refus : impossible de DROP une base à laquelle on est connecté — DATABASE__ADMIN_URL doit viser une base de maintenance (ex. 'postgres')."; exit 1; }; \
	 if [ "$(REFUS_SI_PEUPLEE)" = "1" ]; then \
	     existe=$$(psql "$$admin_url" -tAc "SELECT 1 FROM pg_database WHERE datname = '$$name'" 2>/dev/null) || { \
	         echo "  Erreur : impossible d'interroger $$host avec DATABASE__ADMIN_URL."; \
	         echo "  Refus : on ne détruit pas une base dont on n'a pas pu lire l'état."; exit 1; }; \
	     if [ "$$existe" = "1" ]; then \
	         a_table=$$(psql "$$url" -tAc "SELECT to_regclass('auth__users') IS NOT NULL") || { \
	             echo "  Erreur : la base $$name existe mais n'a pas pu être lue."; \
	             echo "  Refus : une base illisible n'est pas une base vide."; exit 1; }; \
	         if [ "$$a_table" = "t" ]; then \
	             comptes=$$(psql "$$url" -tAc "SELECT count(*) FROM auth__users"); \
	         else comptes=0; fi; \
	     else comptes=0; fi; \
	     [ "$$comptes" = "0" ] || { \
	         echo ""; \
	         echo "  \033[1m\033[31mRefus : la base contient déjà $$comptes compte(s).\033[0m"; \
	         echo ""; \
	         echo "  Cette cible est une **première mise en service**. Elle refuse de"; \
	         echo "  s'exécuter sur une base peuplée : ce n'est pas une précaution"; \
	         echo "  d'usage mais sa définition même."; \
	         echo ""; \
	         echo "  Pour repartir de zéro malgré tout, videz la base à la main —"; \
	         echo "  délibérément, et pas par une commande qui le fait au passage."; \
	         echo ""; \
	         exit 1; }; \
	 fi; \
	 echo ""; \
	 echo "  \033[1m\033[31m/!\\  DESTRUCTION COMPLÈTE DE LA BASE $(LIBELLE)\033[0m"; \
	 echo ""; \
	 echo "     Profil : $(PROFILE)   ($(ENV_FILE))"; \
	 echo "     Hôte   : $$host"; \
	 echo "     Base   : $$name  (owner : $$user)"; \
	 echo ""; \
	 echo "  DROP DATABASE (WITH FORCE — les connexions actives seront coupées)"; \
	 echo "  puis CREATE DATABASE ... OWNER $$user,"; \
	 echo "  migrations et réimport complet."; \
	 echo "  Les comptes, espaces et articles existants seront perdus."; \
	 echo ""; \
	 printf "  Confirmation 1/$(NB_CONFIRMATIONS) — tapez \033[1moui\033[0m pour continuer : "; \
	 read -r answer; \
	 [ "$$answer" = "oui" ] || { echo "  Annulé."; exit 1; }; \
	 printf "  Confirmation 2/$(NB_CONFIRMATIONS) — retapez le nom de la base (\033[1m%s\033[0m) : " "$$name"; \
	 read -r confirm; \
	 [ "$$confirm" = "$$name" ] || { echo "  Annulé — le nom saisi ne correspond pas."; exit 1; }; \
	 if [ "$(CONFIRMER_LIBELLE)" = "1" ]; then \
	     printf "  Confirmation 3/$(NB_CONFIRMATIONS) — tapez \033[1m%s\033[0m : " "$(LIBELLE)"; \
	     read -r prod; \
	     [ "$$prod" = "$(LIBELLE)" ] || { echo "  Annulé."; exit 1; }; \
	 fi; \
	 echo ""; \
	 $(MAKE) --no-print-directory create_remote_db ADMIN_URL="$$admin_url" DB_NAME="$$name" DB_OWNER="$$user" && \
	 $(MAKE) --no-print-directory init_remote_data EXEC_PROFILE=$(PROFILE) DATABASE_URL="$$url" WITH_ACCOUNTS=$(WITH_ACCOUNTS)

# ── Les deux mises en service distantes ──────────────────────────────────────
#
# Même corps, deux garde-fous différents. Ce qui les sépare tient dans les
# variables ci-dessous, et rien d'autre : le fichier d'environnement, le libellé
# des avertissements, et trois drapeaux.
#
#                        démo          production
#   WITH_ACCOUNTS        1             0    pas de seed de comptes en production
#   REFUS_SI_PEUPLEE     0             1    refuse une base qui a déjà des comptes
#   CONFIRMER_LIBELLE    0             1    troisième confirmation
#
# `WITH_ACCOUNTS=0` en production n'est pas un oubli : `scripts/seed_accounts.json`
# porte un mot de passe en clair, versionné dans le dépôt. Les coachs importés du
# legacy conservent leur hachage Django, que la connexion ne sait pas vérifier —
# ils passent tous par « mot de passe oublié », ce qui réécrit leur hachage en
# argon2. C'est connu et assumé.
init_remote_demo_db:
	@$(MAKE) --no-print-directory init_remote_db \
	    PROFILE=$(DEMO_PROFILE) ENV_FILE=$(DEMO_ENV_FILE) LIBELLE="DE DÉMO" \
	    WITH_ACCOUNTS=1 REFUS_SI_PEUPLEE=0 CONFIRMER_LIBELLE=0 NB_CONFIRMATIONS=2

# Première mise en service : elle crée la base de production et y verse les
# données legacy. Elle **refuse** de s'exécuter sur une base qui contient déjà
# des comptes — ce n'est pas une précaution d'usage, c'est sa définition. Rejouée
# par erreur sur une production en service, elle s'arrête au lieu de détruire.
init_remote_prod_db:
	@$(MAKE) --no-print-directory init_remote_db \
	    PROFILE=$(PROD_PROFILE) ENV_FILE=$(PROD_ENV_FILE) LIBELLE="DE PRODUCTION" \
	    WITH_ACCOUNTS=0 REFUS_SI_PEUPLEE=1 CONFIRMER_LIBELLE=1 NB_CONFIRMATIONS=3

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
