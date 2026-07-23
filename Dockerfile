# syntax=docker/dockerfile:1

# Multi-stage : build du binaire, puis image runtime exécutable.
#
# Extraction du seul binaire (sans image runnable) :
#   docker build --target=export --output type=local,dest=./dist .
#   → ./dist/kreek
#
# Image runnable (stage par défaut, voir plus bas pour le run) :
#   docker build -t kreek .

FROM rust:1-slim-bookworm AS builder

RUN apt-get update && apt-get install -y --no-install-recommends \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Couche de cache des dépendances : ne se réinvalide que si Cargo.toml/Cargo.lock changent.
COPY Cargo.toml Cargo.lock ./
RUN mkdir src && echo "fn main() {}" > src/main.rs \
    && cargo build --release \
    && rm -rf src

# Code source réel.
COPY src ./src
COPY askama.toml ./askama.toml
COPY assets/templates ./assets/templates
COPY assets/references ./assets/references
COPY .sqlx ./.sqlx

ENV SQLX_OFFLINE=true

RUN touch src/main.rs && cargo build --release

# Stage d'export — ne contient que le binaire, pas d'entrypoint d'exécution.
FROM scratch AS export

COPY --from=builder /app/target/release/kreek /kreek

# Stage runtime — image exécutable, stage par défaut de ce Dockerfile.
#
# Lancement (pas de préfixe APP__ sur les variables — cf. config.rs, config::Environment
# sans .prefix()) :
#   docker build -t kreek .
#   docker run --rm -p 3210:3210 \
#     -e DATABASE__URL=postgres://user:pass@host/kreek \
#     -e HOST_DOMAIN=... \
#     -e EMAIL__API_KEY=... -e EMAIL__FROM=... -e EMAIL__FROM_NAME=... \
#     kreek
#
# Voir aussi docker-compose.yml pour un lancement basé sur .env.dev.
#
# Exec dans le conteneur (bash disponible) :
#   docker exec -it <container> bash
FROM debian:bookworm-slim AS runtime

RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates \
    libssl3 \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

COPY --from=builder /app/target/release/kreek ./kreek
COPY config ./config
COPY assets/static ./assets/static

EXPOSE 3210

CMD ["./kreek"]