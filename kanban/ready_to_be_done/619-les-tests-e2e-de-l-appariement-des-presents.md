# Les tests e2e de l'appariement des présents

**Épic :** E18 — Appariement des présents · **Ordre :** 8 · **Dépend de :** 618
**Conception :** `docs/specs/appariement-des-presents/calendrier/07-integration.md` (Tests e2e)

## Objectif

Tenir le parcours entier dans un navigateur, de la journée vide aux rencontres
écrites. Le scénario n°9 — « Générer une journée complète » — est livré par la
carte 615.

## Les scénarios — `tests/e2e/test_schedule_draw.py`

1. Journée vide : « Faire les appariements » et « Générer une journée
   complète » visibles ; journée pourvue : aucun des deux (R9).
2. Décocher une équipe met à jour le compteur et la note « impair » ; moins de
   deux présents partout désactive « Tirer au sort » (R2, R10).
3. L'aperçu n'écrit rien : après « Annuler », la journée est toujours vide.
4. « Modifier les présents » conserve les décochages.
5. Écrire : toast de succès, rencontres listées ; les absentes sans match ;
   l'exemptée est celle qui a le plus joué (R8).
6. Une rencontre écrite se supprime, puis s'ajoute à la main (R13).
7. Journée remplie entre l'aperçu et l'écriture : toast d'erreur, rien n'est
   écrit (R11).
8. Deux équipes d'un même coach ne se rencontrent jamais (R4).

**Ce qui n'est pas testé** : l'aléa. « Retirer au sort » est vérifié par un
aperçu valide en retour, jamais par un résultat différent — un test qui dépend
du tirage échoue une fois sur N (cf. cartes 532 et 580).

Cliquer sur le contenu fraîchement injecté passe par
`htmx_helpers.cliquer_quand_cable`.

## Checklist

- [ ] Les huit scénarios
- [ ] Les données : une saison à deux poules, un coach à deux équipes, un
      historique qui force une revanche
- [ ] `tests/impact-map.toml` mis à jour (skill `test-impact`)
- [ ] Suite e2e impactée verte, trois passages de suite
