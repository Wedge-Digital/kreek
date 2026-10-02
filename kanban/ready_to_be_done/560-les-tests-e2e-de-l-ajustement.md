# Les tests e2e de l'ajustement de trésorerie

**Ordre :** 4 · **Dépend de :** `559`
**Conception :** `docs/specs/ajustement-tresorerie/panneau-d-ajustement/07-integration.md`

## Objectif

Prouver sous Playwright ce qu'aucun test unitaire ne voit : que la garde tient
au-delà du JS, et que le solde affiché à deux endroits bouge ensemble.

## Conception

### On étend `test_team_treasury_tab.py`

Pas de fichier neuf. Sa fixture `tresorerie_ctx` construit une compétition
entière — quatre équipes, un match joué et publié, quatre formes de ligne au
relevé. La dupliquer doublerait le coût de la suite pour rebâtir le même
montage.

Et **l'entrée de la carte d'impact ne bouge pas** : `test_team_treasury_tab` y
liste déjà ses neuf BCs. Un fichier neuf en demanderait une qui listerait
exactement les mêmes.

### Les cinq scénarios

| Scénario | Ce qu'il prouve |
|---|---|
| un membre simple ne voit pas le bouton | en HTTP avec `X-Bypass-Auth-Profile: simple`, le markup du panneau est absent du relevé |
| son POST est refusé | **403** — la garde n'est pas qu'à l'affichage |
| un crédit s'applique | le solde monte **dans le bandeau et dans l'en-tête**, et la ligne apparaît en bas avec son motif et son auteur |
| un retrait non couvert est refusé | message dans le pied, panneau toujours ouvert, solde inchangé |
| un motif vide posté en direct est refusé | la garde n'est pas qu'en JS |

**Les deux derniers sont les seuls à prouver ce qu'un test unitaire ne voit
pas** : que les garde-fous Alpine ne sont pas la seule défense. Les trois
premiers éprouvent le câblage.

### Le profil simple

Le middleware n'échange l'identité **que sur une session vide** : en HTTP par
`requests`, sans cookie, l'en-tête suffit. Dans un navigateur, il faudrait un
contexte neuf — raison de plus pour que les deux scénarios de refus soient des
appels HTTP et non des parcours Playwright.

`test_competition_admin_acces.py` pose déjà le motif :

```python
ENTETE_MEMBRE_SIMPLE = {"X-Bypass-Auth-Profile": "simple"}
```

### Le piège du clic

Le scénario du crédit passe par `cliquer_quand_cable` (`htmx_helpers.py`). Le
panneau est injecté par un swap htmx, donc il traverse la fenêtre où un élément
est **peint, cliquable et inerte** : un clic nu s'y perdrait sans requête, sans
erreur de console, sans rien. Pas de `sleep` — la condition rend la main en 7 à
20 ms.

### Deux vérifications dans la base

Le scénario du crédit lit aussi `teams__treasury_ledger` par `query_db` : la
ligne porte-t-elle `AdminAdjustment`, et `balance_after_kpo` concorde-t-il avec
le chiffre affiché ?

**L'écran peut mentir sur une valeur juste** — c'est exactement le défaut des
cartes 492 à 495, qu'aucun test de domaine ne voyait. Lire les deux côtés est le
seul moyen de l'attraper.

### Le POST en direct

Tout POST doit porter `HX-Request: true` : le middleware CSRF maison le refuse
sinon. Les tests du dépôt le posent déjà.

## Checklist

- [ ] Les cinq scénarios dans `test_team_treasury_tab.py`
- [ ] `cliquer_quand_cable` pour le clic sur le panneau, aucun `sleep`
- [ ] Vérification en base du `reason` et de `balance_after_kpo`
- [ ] `HX-Request: true` sur les POST directs
- [ ] `make e2e` au vert, et la carte d'impact vérifiée inchangée
