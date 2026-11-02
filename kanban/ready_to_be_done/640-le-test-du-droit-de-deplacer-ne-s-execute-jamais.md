# Le test du droit de déplacer un match ne s'exécute jamais

**Priorité : moyenne** — aucun défaut produit connu, mais un droit que l'on
croit éprouvé ne l'est pas
**Trouvée par :** la fusion de la carte 634 (2026-11-01), qui a migré ce test
sans qu'il tourne une seule fois.

## Le symptôme

`test_deplacer_un_match.py::test_un_coach_voit_la_correction_mais_pas_le_deplacement`
est **ignoré à chaque exécution** :

```
SKIPPED test_deplacer_un_match.py:230: E2E Coach 01 n'a pas d'équipe dans cette saison
```

Il cherche les équipes du membre simple par la colonne `coach_name` de
`team_proj`. Cette colonne est **vide pour les équipes que montent les tests**
(2324 lignes sur 2387 dans la base e2e au 2026-11-01) : le `SELECT` ne rend
rien, et le test s'ignore lui-même.

Un `skip` ne rougit pas la suite. Le scénario — un coach voit « Corriger le
rapport » mais pas le déplacement, et le widget de déplacement lui est refusé —
n'est donc vérifié par personne, et la règle de correction de la carte 634
(démarreur ou admin) n'y a jamais été éprouvée.

## Ce qu'il faut faire

1. **Trouver le coach par la même voie que `test_match_report_access.py`** : le
   membre simple (« E2E Coach 01 ») est le coach de la **deuxième** équipe que
   monte `build_full_competition` — les coachs y sont pris dans l'ordre
   alphabétique, après DevCoach. Pas de lecture de `coach_name`.
2. **Remplacer le `skip` par un échec** : un décor qui ne permet pas le
   scénario est un défaut du test, pas une raison de le sauter en silence.
3. Garder la migration de la 634 : l'adversaire choisi ne doit pas être coaché
   par DevCoach, sans quoi DevCoach compte comme démarreur et le coach du match
   perd la correction.

## Hors du périmètre

Pourquoi `team_proj.coach_name` est vide pour ces équipes. Si la colonne est
lue ailleurs pour une décision, c'est une autre carte — à ouvrir si la lecture
du code le montre.

## Terminé quand

- [ ] le test s'exécute, et passe, dans `make e2e` ;
- [ ] il échoue — et ne s'ignore plus — si le membre simple n'a pas d'équipe.

## Conséquences sur les données de production

Aucune : seul un test change.
