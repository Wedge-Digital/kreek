# Les tests e2e des coups de pouce

**Épic :** E20 — Coups de pouce : le panier et l'annulation · **Ordre :** 4 · **Dépend de :** 626, 627, 628, 629
**Conception :** `docs/specs/saisie-mobile/E20-cadrage.md` (§ 6)

## Objectif

Le « Terminé quand » de l'épic, éprouvé dans un navigateur : on retire depuis le
panier un article commun sans ouvrir son onglet ; une fois les deux équipes
passées, la page montre leurs achats ; l'annulation les efface, rouvre le choix
du favori, et l'outsider rachète avec un budget recalculé ; après la première
action, l'annulation n'est plus proposée.

Les cartes 627 et 628 reprennent chacune les e2e qu'elles cassent ; celle-ci
ajoute les parcours.

## Conception

Scénarios, chacun sur son propre rapport (une équipe par journée, carte 551 —
cf. la fixture de `test_match_report_inducements.py`) :

| # | Parcours | Ce qui est lu |
|---|---|---|
| 1 | Ajouter un article commun, ouvrir le panier, le retirer d'un ✕ sans changer d'onglet | le compteur, le total, la quantité dans le sélecteur revenue à 0 |
| 2 | Le favori valide ; un second POST pour lui | **409**, `InducementsRecorded` toujours unique dans `match_report_event_store` |
| 3 | Les deux valident ; ouvrir `/inducements/{outsider}` | le récapitulatif, les deux camps, leurs lignes |
| 4 | « Annuler les coups de pouce » → « Garder » | rien n'a changé |
| 5 | « Annuler » → « Oui, tout annuler » | on est sur la page du favori ; la projection porte `NULL` des deux côtés ; les journaliers ont quitté `players_proj` |
| 6 | Après 5, le favori rachète moins ; l'outsider | son budget affiché vaut l'écart de valeur + la nouvelle dépense du favori, plus l'ancienne |
| 7 | Une action enregistrée à l'étape 3, retour aux coups de pouce | plus de bouton d'annulation ; le POST d'annulation répond 409 |

Le scénario 6 compare **trois** valeurs distinctes — budget d'avant, budget
naïf, budget juste —, le patron de `test_inducement_treasury.py:198-225`.

Les clics sur du contenu injecté passent par `cliquer_quand_cable`
(`tests/e2e/htmx_helpers.py`).

## Données de production

Aucune.

## Checklist

- [x] Les sept scénarios
- [x] `tests/impact-map.toml` : le fichier, s'il est nouveau
- [x] La suite des coups de pouce passe en entier : `test_match_report_inducements`,
      `test_inducement_treasury`, `test_inducements_restreints`,
      `test_inducements_tarif_reduit`, `test_cuistot_halfling`,
      `test_match_report_step2_mercenaires`
- [x] `make e2e` impactés
