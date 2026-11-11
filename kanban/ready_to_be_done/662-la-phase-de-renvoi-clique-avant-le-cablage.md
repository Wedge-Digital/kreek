# La phase de renvoi clique avant le câblage

**Priorité : haute** — la suite e2e doit passer à chaque fois (décision du 2026-11-11)
**Épic :** aucune · **Dépend de :** rien
**Trouvée par :** la suite complète de la 652 (2026-11-11) : `test_04` et `test_11`
de `test_dismissals_phase.py` échouent ensemble, et passent seuls.

## Le constat

Le module partage **une seule équipe**, et le panier de renvois vit côté serveur.
Quand `test_11` ne parvient pas à retirer le maillot 1 (ligne 400), le joueur reste
marqué, et `test_04` lit 11 éligibles au lieu de 12 (ligne 415) : un seul défaut,
propagé par le décor partagé.

`test_11` clique le « × » (`retirer.click()`) sans attendre le câblage. Chaque
marquage déclenche trois remplacements : la réponse du POST remplace la liste, puis
`basketChanged` relance un GET de la liste **et** un GET du panier. Une sonde a
mesuré `.cart-remove` non câblé au moment du clic **2 fois sur 10**, en 390 px comme
en 1280 px. `_marquer` vérifie le câblage sur une poignée qui peut être remplacée
aussitôt : un second marquage s'est perdu sans émettre de requête.

**Non prouvé** : l'échec de `test_11` lui-même n'a pas été reproduit. Un `version`
périmé (avis « modifié ailleurs ») reste une explication possible ; le correctif
rend l'échec explicite dans les deux cas.

## Ce qui change (tests seulement)

- Tous les clics nus du fichier passent par `cliquer_quand_cable_locator`
  (scénarios 03, 10, 11).
- `_marquer` attend les trois rendus avant de rendre la main.
- Le clic de retrait est encadré par `page.expect_response(…/unmark)` : l'échec dit
  « aucune requête » ou « refus de version ».
- Une fixture vide le panier par les routes de l'application avant chaque scénario :
  un scénario ne dépend plus de ce que le précédent a laissé.
- Si besoin, `htmx_helpers.py` : cliquer sur la poignée même qui a été attendue.

## Terminé quand

- [ ] le fichier passe **20 fois de suite**, seul, puis précédé d'un fichier lourd ;
- [ ] la sonde de câblage ne trouve plus aucun clic sur un élément non câblé ;
- [ ] la suite complète passe.

## Conséquences sur les données de production

Aucune : seuls des tests changent.
