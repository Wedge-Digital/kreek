# Formatage — rendre `cargo fmt` exécutoire, puis reformater

**Priorité : basse** (aucun impact fonctionnel) — mais le désordre croît tant
qu'elle n'est pas passée
**Dépend de :** —
**Fichiers :** tout `src/`, `.github/workflows/ci.yml`, `.git-blame-ignore-revs`
(nouveau), `CLAUDE.md`

## Problème

`cargo fmt --check` échoue sur **288 fichiers**, ~1487 emplacements. La cible
`make lint` est donc rouge, et l'est depuis longtemps.

Personne ne le voit, et c'est le vrai sujet : **la CI n'exécute ni `make lint`
ni `make check-arch`**. Son `ci.yml` ne lance que `make test` et `make e2e`.
Deux cibles de vérification existent, aucune ne mord.

Reformater sans câbler la vérification, c'est repartir pour la même dérive.
L'ordre des étapes ci-dessous n'est donc pas indifférent : la règle avant le
nettoyage n'aurait aucun sens (la CI serait rouge sur 288 fichiers), le
nettoyage sans la règle non plus.

## Nature du diff

Environ un quart des emplacements sont des réordonnancements d'`use` ; le reste
est du re-découpage de lignes. **Zéro changement de comportement.** Le style
retenu est celui de `rustfmt` par défaut : pas de `rustfmt.toml`, décision
prise le 2026-07-29 — c'est le style que tout Rustacé lit, et un fichier de
configuration invite à des débats de goût sans fin.

## Le piège : `git blame`

Un reformatage de masse fait pointer chaque ligne touchée vers le commit de
formatage au lieu de son auteur réel, sur 288 fichiers. La parade est
`.git-blame-ignore-revs`, que ce dépôt n'a pas.

Elle impose deux contraintes :

- le commit de formatage ne contient **rien d'autre** — pas une correction, pas
  un renommage, pas une carte déplacée ;
- son SHA est ajouté à `.git-blame-ignore-revs` dans le commit **suivant**, un
  commit ne pouvant pas contenir son propre SHA.

## Action, dans cet ordre

1. `cargo fmt` sur tout le dépôt. **Commit seul.**
2. `.git-blame-ignore-revs` avec le SHA de ce commit + la marche à suivre
   (`git config blame.ignoreRevsFile .git-blame-ignore-revs`) documentée dans
   `CLAUDE.md`. À répéter dans `CONTRIBUTING.md` quand ce fichier sera commité.
3. Ajouter `make lint` **et** `make check-arch` au job CI, pour que les deux
   règles mordent. `check-arch` y est absent alors que la règle 9 du CLAUDE.md
   le rend obligatoire avant tout commit — même angle mort, même correctif.

## Point de vigilance

`check-arch.sh` et `select_tests.py` sont des `grep` sur du texte. Un `use`
réordonné ou re-découpé par `rustfmt` peut changer ce qu'ils voient : l'axe 3 et
l'axe 9 exemptent des imports précis (`auth_backend::AuthSession`), et un import
replié sur plusieurs lignes ne matcherait plus le motif. **Relancer
`make check-arch` après le reformatage, pas seulement avant**, et vérifier que
les exemptions tiennent toujours.

## Checklist

- [ ] `cargo fmt` passé, commit isolé ne contenant que du formatage
- [ ] `cargo fmt --check` au vert
- [ ] `.git-blame-ignore-revs` créé avec le SHA du commit de formatage
- [ ] Marche à suivre `blame.ignoreRevsFile` documentée dans CLAUDE.md
- [ ] `make lint` et `make check-arch` ajoutés à `ci.yml`
- [ ] `make check-arch` relancé **après** le reformatage — exemptions des axes 3
      et 9 vérifiées
- [ ] `make test` au vert
- [ ] Suite e2e au vert
