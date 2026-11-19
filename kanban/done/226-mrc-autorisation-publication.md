# `match_report` — Contrôle d'autorisation manquant sur la publication

**Priorité : haute**
**Dépend de :** —
**Fichiers :** `src/app/match_report/io/web/recap_controller.rs`
**Spec :** `docs/specs/match-report-correction/README.md` (dettes préexistantes, n°1)

## Objectif

`post_publish` ne vérifie aujourd'hui que la présence d'un utilisateur connecté.
Contrairement à `get_recap`, il n'appelle jamais `is_authorized()` : tout
utilisateur connaissant un `match_report_id` peut publier le rapport d'autrui.

La règle 4 de la feature de correction aligne les droits de correction sur ceux
de la publication. Sans cette carte, la correction hérite du trou.

## Conception

`is_authorized(&state, &user, &space_id, &source)` attend un `RecapSource`.
`post_publish` charge déjà l'état du rapport pour le use case : le `RecapSource`
est constructible sans coût supplémentaire, via `RecapSource::from_rtp()`.

Ordre dans le handler :

1. utilisateur connecté, sinon `401`
2. `match_report_id` valide, sinon `400`
3. chargement de l'état — `ReadyToPublish` attendu
4. **`is_authorized()`, sinon `403`**
5. use case

L'autorisation vient **après** le chargement de l'état (elle en dépend) mais
**avant** le use case.

Règle des 20 lignes : extraire le chargement + autorisation dans une fonction
dédiée, `post_publish` n'orchestrant que les étapes.

## Checklist

- [ ] `post_publish` appelle `is_authorized()` et renvoie `403` en cas de refus
- [ ] Test : un coach étranger aux 2 équipes reçoit `403`
- [ ] Test : un coach d'une des 2 équipes publie normalement
- [ ] Test : un admin d'espace publie normalement
- [ ] `post_publish` reste sous 20 lignes
- [ ] `make test` passe
- [ ] `make check-arch` passe
