# Les tests e2e de l'onglet Présences

**Priorité : haute — le rendu HTMX ne se vérifie pas autrement**
**Épic :** E16 — Sondage de présence
**Dépend de :** 521
**Fichiers :** `tests/e2e/test_competition_presences.py`, `tests/impact-map.toml`

## Cette carte ne déclare aucune route — et le vérifie

Les douze routes de l'onglet ont été posées par les cartes qui les servent : trois
en 519, une remplie en 520, neuf en 521. Cette carte est la première à les
**traverser toutes** dans un navigateur, donc la première à constater qu'aucune
ne manque ni ne répond à côté.

Son entrée dans `tests/impact-map.toml` va dans le même commit que le test :
l'axe 8 de `check-arch` refuse un test e2e sans entrée, et un test sans entrée est
traité comme `"all"` — donc silencieusement toujours exécuté.

## L'objectif

Le parcours complet dans un navigateur, contre le serveur dev. Aucun test
unitaire ne voit ce que cette carte couvre : le bug du widget coach-search et
celui des pickers de tiers n'ont été trouvés que là.

## Les scénarios

| Scénario | Ce qu'il éprouve |
|---|---|
| lancer une campagne, voir les trois colonnes | R3 — une équipe sans adresse est bien dans « sans réponse » |
| poser une présence à la main, voir le badge | R6 — « saisi par vous » survit au rechargement |
| clore, puis poser encore une réponse | R21 — la clôture ferme le coach, pas l'organisateur |
| tirer avec un seul présent | R15 — le motif s'affiche, le bouton reste inactif |
| tirer, valider, ouvrir le Calendrier | les rencontres y sont, avec leurs projections |
| passer un présent à absent après le tirage | R12 — le panneau de défection propose, il n'agit pas |
| valider la réparation | la rencontre change, **les autres ne bougent pas** |
| vider la journée au Calendrier, revenir en Présences | R24 — l'onglet dit « clos », pas « appariée » |

## Le piège à ne pas rouvrir

**`cliquer_quand_cable` sur tout ce qui vient d'être injecté**
(`tests/e2e/htmx_helpers.py`). Le panneau est remplacé à chaque action : c'est
exactement la fenêtre où un bouton est peint, visible, et **inerte**. Mesuré :
six éléments non câblés à `t=0`, zéro à `t=50ms`.

**Pas de `sleep`.** Une durée fixe n'a aucune marge sur une machine chargée — et
c'est là que la suite échoue — tout en coûtant son délai aux milliers d'appels où
tout est déjà prêt.

## La carte d'impact

`tests/impact-map.toml` est mis à jour **dans le même commit** que le test. Une
carte tests↔BC incomplète fait sauter en silence le test qu'on vient d'écrire —
c'est ce que la carte 480 a trouvé, cinq tests e2e sans entrée que ni le local ni
la CI ne voyaient.

## Checklist

- [ ] Le fichier de test, sur le patron de `test_competition_admin_enrollments.py`
- [ ] Les huit scénarios
- [ ] `cliquer_quand_cable` partout, aucun `sleep`
- [ ] `tests/impact-map.toml` : entrée pour le nouveau fichier
- [ ] `make e2e` passe (serveur dev lancé par l'utilisateur)
- [ ] `make lint`, `make check-arch`, `make test`
