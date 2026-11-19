# Correction d'un rapport — Tests E2E

**Priorité : haute**
**Dépend de :** `232`, `233`, `234`, `235`
**Fichiers :** `tests/e2e/test_match_report_correction.py` (nouveau)
**Spec :** `docs/specs/match-report-correction/recap/07-integration.md`

## Objectif

Couvrir en navigateur ce qu'aucun test unitaire ne peut garantir : le parcours
complet de correction, la propagation réelle des compensations à travers les 5
BCs, et le rendu HTMX de la zone.

## Scénarios

1. Rapport publié, garde-fou passant → le bouton « Corriger » est actif ; après
   correction, le rapport est en `ReadyToPublish` et le bandeau s'affiche
2. L'adversaire achète une compétence → bouton désactivé, message nommant **son**
   équipe (règles 2 et 3)
3. Une équipe valide sa phase d'amélioration → bouton désactivé, message adapté
   (règles 1 et 3)
4. Correction puis re-publication avec un score modifié → le classement reflète
   le **nouveau** score, sans ligne résiduelle de l'ancien (règles 8 et 11)
5. Correction → le match repasse « en cours » dans les résultats de compétition ;
   re-publication → « terminé »
6. Correction → trésorerie et fans de l'équipe restaurés sur sa fiche (règle 14)
7. Deux corrections successives sur le même rapport aboutissent (règle 8)
8. Le bandeau reste visible après modification d'une action puis resaisie de
   l'après-match, sur le parcours réel du navigateur

## Notes

Le **scénario 8** vérifie de bout en bout que le drapeau `was_published_before`
survit à la séquence d'édition qui suit une dépublication.

Sa justification a changé en phase 6 : on croyait que l'édition faisait repasser
le rapport par `PreMatch`, ce qui aurait perdu un drapeau porté par le seul
`ReadyToPublish`. C'est faux — `into_pre_match()` est transitoire et jamais
persistée. Le scénario reste utile, mais il n'est plus le seul filet : le test
unitaire `le_drapeau_survit_a_l_edition_apres_depublication` couvre déjà la
mécanique de rejeu.

Les tests d'écrêtage des fans à 0 et 20 (règle 14) restent **unitaires**, dans la
carte 234 : les provoquer en navigateur demanderait de construire un historique
de fans très long pour un gain de confiance nul.

Les compensations étant asynchrones (app event bus), prévoir des attentes
explicites sur l'état attendu plutôt que des délais fixes.

## Checklist

- [ ] `tests/e2e/test_match_report_correction.py` créé
- [ ] Les 8 scénarios couverts
- [ ] Attentes explicites sur l'état, pas de `sleep` fixe
- [ ] `make e2e` passe (nécessite le serveur dev déjà lancé par l'utilisateur)
