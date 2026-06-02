# Memory Index

- [Commande de test](feedback_test_command.md) — Utiliser `make test`, jamais `cargo test` directement
- [LiveReload + HTMX](feedback_livereload_htmx.md) — Toujours utiliser `request_predicate(NotHtmxRequest)` sinon une SSE par swap → saturation Chrome en dev
- [Pas de commit automatique](feedback_no_auto_commit.md) — Ne committer qu'à la demande explicite de l'utilisateur