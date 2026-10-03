# Le test du journalier recruté est instable — le test était faux, et l'écran aussi

**Priorité : moyenne — un test instable fait douter de toute la suite**
**Épic :** aucune
**Dépend de :** rien
**Fichiers :** `tests/e2e/test_journeyman_recruitment.py`,
`src/app/teams/io/web/view_models.rs`, `src/app/players/routes.rs`,
`src/app/players/io/web/widgets/journeymen_widget.rs`

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

## Le diagnostic (2026-10-03)

Une sonde a suivi le clic sur « Recruter » : le POST part, sa réponse émet
`basketChanged`, le catalogue se recharge, puis le panneau des journaliers. La
chaîne fonctionne. Et le panneau **reste** — le journalier est toujours
`Journeyman` en base, ce qui est juste tant que la phase n'est pas validée.

**Le test attendait une chose fausse.** Il voulait voir le panneau disparaître.
Il ne passait qu'en tombant, par chance, dans les quelques millisecondes où le
catalogue rechargé n'a pas encore remonté le panneau. D'où l'intermittence,
seul comme en suite, et sans rapport avec la charge.

**L'écran avait un défaut que l'intermittence cachait.** La maquette veut « Au
panier » sur un journalier déjà pris (`app-team-recruitment.html`). Le domaine
le sait — `action_for_journeyman` rend `JourneymanAlreadyInBasket`. Mais
`url_du_panneau_journaliers` ne parcourait que `hireable_journeymen()`, qui
écarte ceux du panier : leur motif n'arrivait jamais au widget, qui rendait un
bouton grisé **sans un mot**.

## Le changement

- `teams` passe au widget les journaliers du panier, dans un paramètre
  `au_panier` à part — pas dans le motif unique : « au panier » et
  « trésorerie insuffisante » peuvent coexister sur deux journaliers.
- `players` les rend « Au panier ». La décision du blocage est extraite dans
  `blocage()`, que le test unitaire appelle au lieu de la recopier.
- Le test e2e attend ce qui est vrai : « Panier · 1 », et le bouton du
  journalier qui dit « Au panier », désactivé. L'attente reste nécessaire —
  sans elle, la validation peut partir avant l'ajout, sur un panier vide.
