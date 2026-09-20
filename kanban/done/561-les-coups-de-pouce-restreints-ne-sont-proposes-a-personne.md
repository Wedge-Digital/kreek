# Les coups de pouce restreints ne sont proposés à personne

**Priorité : haute — quatre coups de pouce du corpus sont inaccessibles à toutes les équipes**
**Épic :** aucune — une correction, livrable d'un bloc
**Dépend de :** 560
**Fichiers :**
`src/app/references/domain/profil_roster.rs` *(nouveau)*,
`src/app/references/domain/inducement_availability.rs` *(nouveau)*,
`src/app/references/domain/inducement_pricing.rs`, `mod.rs`,
`src/app/references/io/web/inducement_selector_controller.rs`,
`src/infrastructure/match_report/competition_data_adapter.rs`,
`assets/references.example/inducements_fr.json`,
`tests/e2e/test_inducements_restreints.py` *(nouveau)*,
`tests/e2e/test_inducement_treasury.py`, `tests/impact-map.toml`

## L'objectif

Un coup de pouce restreint est proposé aux équipes qui y ont droit, et à
elles seules. Au sélecteur comme à l'achat.

## Ce qui l'a fait naître

Trouvé en instruisant la carte 560, dans le même fichier.

Le sélecteur écartait un coup de pouce dont le `restrictedTo` ne contenait pas
**l'identifiant du roster**. Or le corpus y met trois choses différentes :

| Coup de pouce | `restrictedTo` | Ce que c'est |
|---|---|---|
| Assistant mortuaire | `MASTERS_OF_UNDEATH` | une règle spéciale |
| Médecin de peste | `FAVOURED_OF_NURGLE` | une règle spéciale |
| Recrues turbulentes | `LOW_COST_LINEMEN` | une règle spéciale |
| Apothicaire itinérant | `APOTHECARY` | un membre du **staff** |

Aucune n'est un roster. Sur le corpus de production, **les quatre n'étaient
donc proposés à personne**, quelle que soit l'équipe.

**Le jeu de démonstration ne portait que la quatrième forme** — le Masseur
douteux, restreint au roster `DEMO_GRANIT`, la seule que le filtre savait lire.
C'est ce qui a fait passer le défaut inaperçu : les tests exerçaient le seul
cas qui marchait.

**Et le droit ne vivait qu'à l'affichage.** La liste des coups de pouce
autorisés que l'adapter donne au domaine ne filtrait rien : un achat forgé à la
main passait, même sur un coup de pouce que l'équipe n'a pas le droit de
prendre.

## Le changement

**Une entrée de `restrictedTo` est satisfaite si elle désigne le roster, l'une
de ses règles spéciales, ou un membre du staff qu'il peut engager.** Les trois
espaces d'identifiants ne se recoupent pas — 31 rosters, 14 règles, 6 membres
du staff, aucune collision — donc tester les trois n'ouvre aucun droit par
homonymie.

**Le staff autorisé, et non le staff engagé** : l'Apothicaire itinérant est
réservé aux équipes qui *ont le droit* d'avoir un apothicaire. Quatre n'en ont
pas le droit : Rois des Tombes, Horreur nécromantique, Nurgle, Morts Ambulants.

**Le même filtre des deux côtés**, comme la carte 507 l'a établi pour le prix :
ce qu'on montre et ce qu'on accepte sortent du même endroit.

**`ProfilRoster` passe dans son propre module.** La carte 560 l'avait posé
dans la tarification, son seul consommateur d'alors ; la disponibilité en est
un second, et il gagne le staff autorisé.

## Tests

Unitaires, sur les trois formes : une règle spéciale ouvre le droit, un membre
du staff aussi, un roster nommé toujours — plus un coup de pouce sans
restriction ouvert à tous, un roster inconnu du catalogue fermé à tout, et une
entrée satisfaite parmi plusieurs qui suffit.

E2E, `test_inducements_restreints.py` : ce que le sélecteur propose et ce que
l'achat accepte, sur une équipe qui a droit à un apothicaire et une qui n'y a
pas droit. Le jeu de démonstration reçoit un Guérisseur Itinérant restreint au
staff `APOTHECARY`, la forme qu'il ne portait pas.

**`test_inducement_treasury.py` change de panier.** Il faisait acheter deux
Masseurs douteux à l'underdog, qui est celle des deux équipes qui vaut le
moins — donc parfois `DEMO_ZEPHYR`, qui n'y a pas droit. Il passait parce que
l'achat ne vérifiait rien. Trois Renforts remplacent les deux Masseurs, à
somme égale : le test porte sur la trésorerie, pas sur les restrictions.

## Terminé quand

Une équipe Nurgle se voit proposer le Médecin de peste, une équipe Morts
Ambulants ne se voit pas proposer l'Apothicaire itinérant, et une requête
forgée pour l'acheter quand même est refusée.
