# L'onglet Ajout direct sous Playwright

**Priorité : haute** — c'est le seul filet du contrat entre `auth` et `spaces`
**Dépend de :** 376 à 383
**Conception :** `docs/specs/space-admin/ajout-direct/07-integration.md`
**Fichiers :** `tests/e2e/test_space_admin_direct_add.py`

## Les scénarios

| Scénario | Vérifie |
|---|---|
| chercher un coach existant | la liste, l'email affiché |
| chercher un membre de l'espace | badge « Déjà membre », pas de bouton |
| ajouter un coach | la ligne passe en « Déjà membre », le compteur monte, la liste des membres le contient |
| l'ajouté apparaît au journal **immédiatement** | le test de la course du cache |
| le retirer depuis le journal | la ligne disparaît, le compteur redescend |
| chercher un pseudo inexistant | état vide **et** invitation à créer un compte |
| **créer un compte et ajouter** | le compte existe, le coach est membre, le journal l'affiche |
| taper un seul caractère | état sous-seuil, sans invitation à créer |

## Le septième est la raison d'être de cette carte

`accountCreated`, `coach_id`, `name` — trois chaînes qui franchissent la
frontière entre `auth` et `spaces` par le navigateur. **Rien d'autre ne les
vérifie** : ni le compilateur, ni `cargo test`, ni `check-arch`, qui est un
`grep` aveugle aux chaînes littérales et aux attributs HTML.

Si `auth` renomme `coach_id` en `id`, tout compile, tous les tests unitaires
passent, et l'ajout cesse silencieusement de fonctionner.

Le scénario doit donc vérifier **la chaîne complète** — compte créé *et*
appartenance posée — pas seulement que le formulaire répond.

## Le quatrième vérifie une conception, pas un affichage

Le journal affiche depuis le payload de `memberAdded`, sans relire. C'est ce qui
masque le délai d'alimentation de `spaces__user_cache`. Si quelqu'un
« simplifie » plus tard en le faisant relire, ce test rougira — et c'est tout ce
qui protège la décision.

## Le piège des tests d'échange HTMX

Un clic sur un élément que sa propre requête remplace peut être **rejoué par
Playwright** : `click()` vérifie l'actionnabilité pendant l'action et recommence
si l'élément disparaît sous lui. Vécu sur `test_dismissals_phase`.

Le remède n'est **pas** `dispatch_event`, qui court-circuite l'actionnabilité et
clique parfois trop tôt : c'est d'attendre l'état réel après chaque action.

Ici l'ajout rafraîchit **trois zones** — la ligne candidate, le journal, les
statistiques. L'attente porte sur les trois.

## Checklist

- [x] Les huit scénarios
- [x] Chaque action attend l'**état résultant**, jamais une durée
- [x] Aucun `dispatch_event`
- [x] Suite lancée **cinq fois** sans échec — et la suite de l'onglet Membres
      relancée, non régressée
- [x] `make lint`, `make check-arch`, `make test` passent — 1203 tests

## Ce qu'on a appris en la faisant

**Deux défauts que rien d'autre ne pouvait voir.**

`htmx.process` manquait sur les panneaux d'onglet : **HTMX ne connaît pas le
markup inséré par Alpine**, donc les `hx-trigger="load"` d'un onglet monté au
clic ne partaient jamais, et l'Ajout direct s'affichait vide.

L'onglet Membres, lui, **fonctionnait par accident** — monté dès l'initialisation
d'Alpine, il était encore là quand HTMX a parcouru la page. Les autres arrivaient
trop tard. C'est le genre de dépendance à l'ordre qui tient jusqu'au jour où elle
ne tient plus.

**Une omission de la carte 369**, posée dès la conception : le widget des membres
devait écouter `memberAdded`, et ne l'écoutait pas. Rebasculer sur l'onglet ne le
recharge pas — c'est voulu — donc la liste restait périmée après un ajout fait
depuis l'autre onglet.

Aucun test unitaire ni de harnais ne pouvait voir ces deux-là.

**Le contrat entre les deux BCs est enfin fermé.** Le scénario « créer un compte
et ajouter » vérifie la chaîne complète — compte présent en base **et**
appartenance posée. Les tests de harnais des cartes 380 et 383 vérifient chacun
un bord ; seul celui-ci vérifie qu'ils s'accordent.
