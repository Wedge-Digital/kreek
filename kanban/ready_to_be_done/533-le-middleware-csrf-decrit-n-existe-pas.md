# Le middleware CSRF décrit n'existe pas

**Priorité : moyenne — la protection tient, sa description est fausse**
**Dépend de :** rien · **Sans épic**
**Trouvée par :** l'investigation du sondage de présence (E16, phase 1)
**Fichiers :** `CLAUDE.md`, `src/main.rs`, `src/app/.../test_harness.rs`

## Le constat

Trois endroits décrivent « un middleware CSRF maison qui exige `HX-Request:
true` sur toute mutation » : le `CLAUDE.md`, le commentaire de `main.rs` sur la
configuration du cookie, et celui du harnais de test qui pose l'en-tête « pour
lui ».

**`grep -ri csrf src/` ne trouve que ces commentaires.** Aucun fichier ne
l'implémente.

## Ce qui protège réellement

Le cookie seul. `SameSite::Lax` n'est pas envoyé sur une requête `POST` venue
d'un autre site, ce qui suffit pour les mutations.

**Le mécanisme fonctionne ; c'est sa description qui est fausse.** Et une
description fausse est précisément ce qui fait qu'on retire un jour la vraie
garde en croyant l'autre en place — ici, quelqu'un qui passerait le cookie en
`SameSite::None` pour régler un problème d'intégration, rassuré par un
middleware qui n'existe pas.

## À trancher dans la carte

Deux issues, et le choix n'est pas évident :

1. **Corriger la description** — trois commentaires et une section du
   `CLAUDE.md` disent alors que la garde est le cookie. Coût : une heure.
2. **Écrire le middleware** — la garde décrite devient réelle, et la protection
   ne dépend plus d'un seul mécanisme.

L'argument pour (1) : la protection actuelle est correcte, et un second verrou
qui n'ajoute rien contre un attaquant réel coûte un middleware à maintenir.
L'argument pour (2) : `SameSite` est une garde du navigateur, et le jour où une
route doit accepter une requête inter-sites, elle tombe pour tout le monde.

**Ne pas laisser en l'état** est la seule chose qui ne se discute pas.

## Checklist

- [ ] Trancher entre (1) et (2), et écrire le motif dans la carte
- [ ] Appliquer, y compris la section « Middleware — ordre d'exécution » du `CLAUDE.md`
- [ ] Si (1) : le commentaire du harnais de test doit dire pourquoi il pose
      l'en-tête, puisque ce n'est plus « pour le middleware »
- [ ] `make lint`, `make check-arch`, `make test`
