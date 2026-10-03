# Le test du journalier recruté est instable

**Priorité : moyenne — un test instable fait douter de toute la suite**
**Épic :** aucune
**Dépend de :** rien
**Fichiers :** `tests/e2e/test_journeyman_recruitment.py` (à confirmer après
diagnostic)

## Le symptôme

`test_journeyman_recruitment.py::test_le_journalier_recrute_reste_dans_l_effectif`
échoue par intermittence :

```
expect(page.locator(".panel--jm")).to_have_count(0, timeout=10000)
AssertionError: Locator expected to have count '0'
Actual value: 1
```

Après le clic sur le bouton de recrutement du journalier, le panneau des
journaliers est **toujours là** dix secondes plus tard — il devait se recharger
avec le catalogue, journalier sorti.

## Ce qu'on en sait (2026-10-03)

- Vu **deux fois dans une suite complète** (avant la carte 568, puis pendant la
  validation des cartes 570 à 572), jamais ailleurs — avant même que les droits
  d'administration ne changent.
- Lancé seul : deux succès sur trois, puis un échec sur trois. Ce n'est donc pas
  la charge de la suite seule.
- Le clic passe par `cliquer_quand_cable_locator` : la fenêtre « visible mais pas
  encore câblé » (`htmx_helpers.py`, `CLAUDE.md`) devrait être couverte. Si
  c'est quand même elle, le helper ne couvre pas ce cas ; sinon, c'est le
  rechargement du panneau qui manque son déclencheur.

## À faire

Diagnostiquer avant de corriger : savoir si la requête de recrutement part
(`page.expect_request`), si sa réponse arrive, et si l'événement qui recharge le
panneau est émis. **Pas de `sleep`** — la règle de `htmx_helpers.py` tient.

## Terminé quand

Le test passe vingt fois de suite seul, et la suite complète passe sans lui
accorder de seconde chance.
