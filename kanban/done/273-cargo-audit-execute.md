# Audit des dépendances — le faire tourner, et savoir ce qu'il trouve

**Priorité : moyenne** — aucun impact fonctionnel, mais c'est une garantie
affichée qui n'existe pas
**Dépend de :** 272 (qui a branché `make lint` et `make check-arch` en CI)
**Fichiers :** `Makefile`, `.github/workflows/ci.yml`,
`.github/dependabot.yml` (nouveau), `.cargo/audit.toml` (éventuel)

## Problème

`make lint` annonce une étape « Audit des dépendances » qui **ne s'exécute
jamais**. `cargo-audit` n'est installé ni en local ni sur le runner, et le
Makefile passe outre :

```make
@if command -v cargo-audit >/dev/null 2>&1; then \
    cargo audit && echo "  ✓ PASS  cargo audit"; \
else \
    echo "  ⚠ SKIP  cargo audit non installé"; \
fi
```

Le `else` **n'échoue pas**. Le job `qualite` est donc vert en ayant sauté
l'étape — exactement le motif de la carte 272, à ceci près qu'ici la cible
tourne dans le vide au lieu de ne pas tourner du tout. C'est pire : elle affiche
une ligne rassurante.

**398 dépendances pour 27 directes.** Le risque est essentiellement transitif —
on ne choisit pas les crates qu'`axum`, `sqlx` et `tokio` tirent derrière eux —
et personne ne le regarde.

## Une incohérence à élucider d'abord

GitHub affiche à chaque `push` :

```
remote: GitHub found 4 vulnerabilities on Wedge-Digital/kreek's default branch
        (1 high, 2 moderate, 1 low)
```

Or `gh api repos/Wedge-Digital/kreek/dependabot/alerts` renvoie une **liste
vide**, sans erreur de permission. **On ne sait donc pas ce que sont ces quatre
vulnérabilités.** Établir ce fait est le préalable : tout le reste de la carte
en dépend, et décider d'un seuil de blocage sans ces données n'aurait pas de
sens.

## Alertes et mises à jour sont deux mécanismes distincts

Confusion fréquente, à ne pas reproduire ici :

| | Quoi | Où ça se règle |
|---|---|---|
| **Dependabot alerts** | signale les vulnérabilités connues | réglage du dépôt, pas un fichier |
| **Dependabot version updates** | ouvre des PR de montée de version | `.github/dependabot.yml` |
| **`cargo audit`** | même base d'avis, mais en local et en CI | `Makefile`, `ci.yml` |

Le dépôt n'a **aucun `.github/dependabot.yml`** : aucune PR de montée de version
n'est proposée aujourd'hui.

## Action

1. **Établir ce que `cargo audit` trouve réellement**, en local
   (`cargo install cargo-audit`). C'est le préalable.
2. **Confronter au signalement de GitHub** et comprendre l'écart avec l'API.
   Les deux bases d'avis — RustSec et GitHub Advisory — se recouvrent largement
   sans être identiques.
3. **Traiter ce qui doit l'être** : montée de version, ou `.cargo/audit.toml`
   avec une **justification écrite par avis ignoré**. Un avis ignoré sans motif
   est un avis oublié.
4. **Installer `cargo-audit` dans le job `qualite`**, via
   `taiki-e/install-action` — déjà utilisé pour `sqlx-cli`, binaire précompilé.
5. **Faire échouer le SKIP en CI.** C'est le cœur de la carte : tant que
   l'absence du binaire reste gagnante, réinstaller ne garantit rien — il suffit
   qu'il disparaisse du runner pour revenir au vert silencieux. En local le SKIP
   peut rester tolérant ; en CI, non.
6. **Ajouter `.github/dependabot.yml`** avec les écosystèmes `cargo` **et**
   `github-actions` — les cinq actions du workflow (`checkout`,
   `rust-toolchain`, `rust-cache`, `install-action`, `setup-uv`) ne sont pas
   suivies non plus.

## Décision à prendre en cours de route

**Bloquer sur toute vulnérabilité, ou au-dessus d'un seuil ?** À trancher une
fois l'étape 1 faite. Trois avis sur des crates transitives non atteignables
n'appellent pas la même réponse qu'une faille dans `axum`. Décider avant de
savoir reviendrait soit à bloquer la CI sur du bruit, soit à se donner un seuil
qui laisse tout passer.

## Checklist

- [ ] `cargo audit` exécuté en local, résultat consigné dans la carte
- [ ] Écart avec le signalement Dependabot élucidé
- [ ] Vulnérabilités traitées, ou ignorées avec justification écrite
- [ ] `cargo-audit` installé dans le job `qualite`
- [ ] **Le SKIP fait échouer la CI**, tout en restant tolérant en local
- [ ] `.github/dependabot.yml` — écosystèmes `cargo` et `github-actions`
- [ ] Seuil de blocage tranché explicitement
- [ ] `make lint` au vert avec l'audit **réellement exécuté**
