# Le tarif réduit de « Chantage et Corruption »

**Priorité : haute — cinq rosters paient deux coups de pouce au double de leur prix**
**Épic :** aucune — une correction, livrable d'un bloc
**Dépend de :** rien
**Fichiers :**
`src/app/references/domain/models.rs`,
`src/app/references/domain/inducement_pricing.rs`,
`src/app/references/io/web/inducement_selector_controller.rs`,
`src/infrastructure/match_report/competition_data_adapter.rs`,
`assets/references.example/inducements_fr.json`,
`tests/e2e/test_inducements_tarif_reduit.py` *(nouveau)*, `tests/impact-map.toml`

## L'objectif

Une équipe qui porte la règle spéciale **Chantage et Corruption** achète ses
Pots-de-vin 50 kPo au lieu de 100, et son Représentant véreux de la ligue
80 kPo au lieu de 120. Au prix affiché comme au prix débité.

## Ce qui l'a fait naître

Un coach l'a signalé sur les pots-de-vin. La règle n'était appliquée nulle part.

Le corpus la porte pourtant : `BRIBES` a `cost: 100`, `reducedCost: 50` et
`reducedCostFor: ["BRIBERY_AND_CORRUPTION"]`. Mais **`reducedCostFor` n'existait
pas dans la struct `Inducement`** et disparaissait à la désérialisation. La
tarification ne connaissait qu'une règle, écrite en dur par la carte 507 : le
cuisinier halfling.

**Deux coups de pouce, et non un seul** — le second a la même règle et le même
défaut :

| Coup de pouce | Plein tarif | Tarif réduit |
|---|---|---|
| Pots-de-vin | 100 | 50 |
| Représentant véreux de la ligue | 120 | 80 |

Cinq rosters portent la règle : Nains, Habitants des Bas-Fonds, Orques Noirs,
Gobelin, Snotlings. Vingt compétitions de la base de production autorisent ces
deux coups de pouce dans leurs tiers.

**Rien à réparer dans les données** : un seul achat de pot-de-vin est
enregistré, par une équipe Morts Ambulants, qui n'a pas la règle. Ses 100 kPo
étaient justes.

## Le changement

**Le corpus nomme la règle, on la lit.** `Inducement` désérialise
`reducedCostFor`, et le tarif réduit s'applique quand le roster porte l'une des
règles spéciales qui y figurent. Deux paires de plus en dur auraient marché ;
elles auraient aussi fallu être réécrites au prochain coup de pouce réductible.

**Un profil de roster plutôt qu'un identifiant.** `cout_pour_roster` recevait
un `roster_id` seul, qui ne dit rien des règles spéciales. Elle reçoit
désormais un `ProfilRoster`, emprunté au catalogue par les deux appelants.

**Le cuisinier halfling reste en dur**, et c'est la partie à ne pas défaire.
Son `reducedCostFor` désigne `HALFLING_THIMBLE_CUP`, qui est une **ligue** et
non une règle spéciale : la brancher donnerait le tarif réduit aux Gnomes, qui
partagent cette ligue sans y avoir droit. C'est l'erreur que la première
analyse de la carte 507 avait commise, et son commentaire l'explique.

Le nouveau mécanisme ne peut pas le déclencher par accident : aucun identifiant
de ligue ne coïncide avec un identifiant de règle spéciale, et un roster ne
porte jamais une ligue dans ses règles spéciales. Vérifié sur le corpus, dix
ligues et quatorze règles, aucune collision.

**Les deux appelants, toujours.** Le prix affiché par le sélecteur et le prix
débité par le rapport de match sortent de la même fonction. C'est ce que la
carte 507 a établi, et pour la même raison : corriger le seul affichage ferait
lire 50 au coach en lui prélevant 100.

## Tests

Unitaires : le tarif réduit sur un roster qui porte la règle, le plein tarif
sur un roster qui ne la porte pas, les deux coups de pouce concernés, le
cuisinier halfling inchangé dans les deux sens, et un roster inconnu du
catalogue qui paie plein tarif.

E2E, `test_inducements_tarif_reduit.py` : le prix affiché par le sélecteur et
le prix débité par l'achat, sur un roster qui porte la règle et sur un témoin
qui ne la porte pas. Bâti sur `test_cuistot_halfling.py`, qui éprouve l'autre
règle de tarif du même module.

**Le corpus de production n'est pas versionné**, et le jeu de démonstration ne
portait aucun `reducedCostFor`. Le Renfort Temporaire en reçoit un : 20 kPo,
10 pour un roster qui porte `LOW_COST_LINEMEN`, que seul `DEMO_LANTERNE` a.
Même mécanisme que « Chantage et Corruption », exercé de bout en bout.

Ce roster-là, et pas un autre : `DEMO_LANTERNE` est hors du cycle de
`build_full_competition`, donc les prix que `test_inducement_treasury` épingle
ne bougent pas. Donner la règle à un roster du cycle aurait fait échouer ce
test sans rapport avec la carte.

## Terminé quand

Sur l'écran des coups de pouce, une équipe Gobelin voit ses Pots-de-vin à
50 kPo, et sa trésorerie est débitée de 50 kPo à l'achat.
