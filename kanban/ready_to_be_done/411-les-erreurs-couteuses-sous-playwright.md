# Les erreurs coûteuses sous Playwright

**Priorité : haute**
**Dépend de :** 410
**Conception :** `docs/specs/erreurs-couteuses/ecran-du-jet/07-integration.md`
**Fichiers :** `tests/e2e/`, la carte d'impact du skill `test-impact`

## Les six scénarios

| # | Scénario | Vérifie |
|---|---|---|
| 1 | Renvois validés à **99 kPo** → l'équipe est prête à jouer, **aucun écran** | le seuil |
| 2 | Renvois validés à **150 kPo** → le bandeau propose « Lancer le dé » | le seuil, l'accès |
| 3 | Lancer → le résultat s'affiche, la trésorerie de la fiche a baissé du montant annoncé | chemin nominal |
| 4 | **Relancer en contournant le bouton** → 409, la trésorerie n'a pas rebougé | un seul jet |
| 5 | Un coach tiers ouvre l'URL du jet → 403, aucun événement | le droit |
| 6 | Ouvrir la page hors phase → 422 | la garde de page |

## Le quatrième est la raison d'être de cette carte

Un double jet **retirerait de l'argent deux fois**. C'est le genre de défaut
qu'un utilisateur découvre avant nous, et il ne se teste pas en cliquant : le
bouton est désactivé après le premier jet. Il faut **poster deux fois**, sans
passer par l'interface.

Le premier scénario, lui, vérifie une **absence d'écran** — ce qu'aucun test
unitaire ne peut voir : la logique serveur est identique, seule la redirection
change.

## Le jeu de données

Il faut deux équipes en phase de renvois, l'une sous le seuil et l'autre
au-dessus, et un coach tiers pour le cinquième scénario. **À vérifier avant
d'écrire** : le jeu e2e porte-t-il déjà de quoi placer une équipe dans cette
phase avec une trésorerie choisie ?

## Le dé est aléatoire — et c'est le piège de cette carte

Le serveur tire pour de vrai. Un test qui attendrait « incident majeur » serait
**instable une fois sur six**.

Les scénarios ne doivent donc porter que sur ce qui ne dépend pas du jet : qu'un
résultat s'affiche, que la trésorerie affichée **corresponde au montant annoncé à
l'écran**, qu'un second jet soit refusé. La table, elle, est vérifiée par les 36
tests unitaires de la carte 408 — c'est leur raison d'être.

## Ne pas oublier la carte d'impact

Le skill `test-impact` tient une carte tests ↔ bounded contexts. **Un nouveau
test e2e impose sa mise à jour**, sans quoi il ne sera jamais sélectionné par les
exécutions ciblées et ne tournera qu'en CI complète.

## Checklist

- [ ] Les six scénarios dans `tests/e2e/`
- [ ] Aucune assertion sur l'issue du jet — seulement sur la cohérence
      écran / trésorerie
- [ ] Jeu de données : deux équipes de part et d'autre du seuil, un coach tiers
- [ ] Carte d'impact tests ↔ BC mise à jour
- [ ] Chaque test **vu échouer** avant d'être vu passer
- [ ] `make e2e` complet au vert
