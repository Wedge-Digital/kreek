# `match_reminders_fr.json`, premier fichier optionnel du corpus

**Priorité : moyenne**
**Épic :** E16 — Imprimer la feuille d'équipe · PR 2
**Dépend de :** rien
**Spec :** `README.md` (« Un nouveau fichier optionnel du corpus »), `04-dtos.md` (`MatchRemindersFile`), R9
**Fichiers :** `src/app/references/io/repository/in_memory_reference_repository.rs`, `src/app/references/domain/port.rs`, `assets/references.example/match_reminders_fr.json`, `docs/reference-data-schema.md`

## Objectif

Les rappels du MVP et des fans, et la liste des météos, sont des règles de jeu
que kreek ne code pas (`CONTRIBUTING.md`). Ils arrivent par le corpus, dans un
fichier **optionnel** : un corpus qui ne l'a pas — le catalogue de production
aujourd'hui — démarre comme avant.

## Conception

- `MatchRemindersFile { mvp: Option<String>, fans: Option<String>, weather: Vec<String> }`,
  champs `#[serde(default)]`.
- Lecture optionnelle à côté de `read_json` : fichier absent →
  `Default` ; mal formé → erreur de démarrage nommant le fichier.
- `IReferenceRepository::match_reminders()`.
- Version fictive dans `assets/references.example/`.
- `docs/reference-data-schema.md` : section du fichier, **et** correction des
  lignes fausses — le chargeur exige douze fichiers et non neuf,
  `spp_rules.json` compris (il est dit « non chargé »).

## Checklist

- [ ] Lecture optionnelle + tests : absent → vide · présent → lu · mal formé → erreur nommant le fichier
- [ ] Fichier fictif ; `load_for_tests` le lit
- [ ] Doc du schéma : nouveau fichier et correction des obligatoires
- [ ] À signaler dans la PR : les vrais textes restent à ajouter au catalogue de production
- [ ] `make check-arch`, `make lint`, `make test`
