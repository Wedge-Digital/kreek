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

## Un septième scénario, écrit en plus des six

Le sixième de la table — « ouvrir la page hors phase » — est vérifié tel quel.
S'y ajoute **une équipe inconnue** : `404` sur la page comme sur le jet. Le
handler charge l'équipe pour décider du droit, et un identifiant qui ne
correspond à rien y passe par une branche que rien d'autre n'emprunte.

Ce n'est pas un caprice de couverture. Les six scénarios de la table portent
tous sur une équipe qui existe : sans ce septième, un handler qui répondrait
`500` — ou pire, `403`, ce qui confirmerait l'inexistence par la négative — ne
serait vu par aucun d'eux.

## Ce que la suite complète a révélé — quatre tests que l'épic avait cassés

`make e2e` au vert n'était pas une formalité de fin de carte : **la suite était
rouge avant qu'on l'écrive**, et les cartes 408 à 410 étaient déjà commitées et
poussées. Onze tests tombaient, tous sur la même cause.

La phase des erreurs coûteuses **s'intercale** entre les renvois et « prête à
jouer ». Tout test qui validait des renvois puis attendait `ReadyToPlay`
attendait désormais en vain :

| Fichier | Ce qui cassait |
|---|---|
| `test_roster_edition.py` | 8 erreurs — sa fixture n'atteignait plus `ReadyToPlay`, aucun de ses scénarios ne démarrait |
| `test_dismissals_phase.py` | scénario 6 (phase attendue) et 9 par ricochet |
| `test_team_detail_state_banner.py` | la chaîne de bandeaux s'arrêtait avant la fin |

**Le gain de match par défaut vaut 50 000 kPo.** Toute équipe de test dépasse
donc le seuil de 100 : la phase n'est contournable pour aucune d'elles.

### Deux réparations, pas une

Elles n'ont pas le même statut, et les confondre aurait fait perdre de la
couverture :

- `test_team_detail_state_banner.py` **suit la séquence de bout en bout par de
  vraies actions** — c'est sa raison d'être. Il a donc été **étendu** : bandeau
  « Lancer le dé → », écran, jet, lien de sortie, « Prête à jouer ». La nouvelle
  phase entre dans la chaîne qu'il documente.
- Les deux autres ne portent pas sur elle. Ils la **franchissent** via
  `tests/e2e/team_phase_helpers.py`, module partagé écrit pour l'occasion.

### Le piège du franchissement

`traverser_erreurs_couteuses` ne promet **rien** sur la trésorerie qu'il laisse :
une catastrophe ne laisse que 2d6 dizaines de kPo, soit 20 au pire. Un scénario
qui dépense après l'avoir appelé serait instable — pas toujours, ce qui est pire.

Le scénario 9 de `test_dismissals_phase` dépense justement, et il est sûr pour
une raison qu'il fallait vérifier et non supposer : **le match qu'il joue
entre-temps rapporte 50 000 kPo**, et son recrutement n'a lieu qu'après. C'est
écrit dans le module comme dans le test.

## Checklist

- [ ] Les six scénarios dans `tests/e2e/`
- [ ] Aucune assertion sur l'issue du jet — seulement sur la cohérence
      écran / trésorerie
- [ ] Jeu de données : deux équipes de part et d'autre du seuil, un coach tiers
- [ ] Carte d'impact tests ↔ BC mise à jour
- [ ] Chaque test **vu échouer** avant d'être vu passer
- [ ] `make e2e` complet au vert
