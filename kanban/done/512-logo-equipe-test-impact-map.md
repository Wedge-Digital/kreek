# Mettre à jour la carte d'impact tests e2e

**Priorité : basse — mais obligatoire au même commit que le premier test e2e ajouté**
**Dépend de :** carte 511
**Fichiers :** `tests/impact-map.toml`, nouveau test e2e de la carte 511

## Règle rappelée (`.claude/skills/test-impact/SKILL.md`)

Tout nouveau test e2e doit être ajouté à `impact-map.toml` **dans le même
commit**, en listant les BCs réellement traversés (routes appelées, tables
lues) — jamais deviné depuis le nom du fichier. `make check-arch` (axe 8)
échoue si la carte décroche du code.

## Plan

Lire le nouveau test e2e (carte 511) une fois écrit, lister précisément :
`teams` (route logo, projection), probablement rien d'autre si le test ne
traverse pas de fixture d'un autre BC pour poser son équipe de test.

## Checklist

- [ ] Entrée ajoutée dans `impact-map.toml` pour le nouveau test
- [ ] `make check-arch` (axe 8) passe
- [ ] Backtest rapide : `scripts/impact/changed_bcs.sh` sur le diff de la
      carte 511 sélectionne bien ce nouveau test
