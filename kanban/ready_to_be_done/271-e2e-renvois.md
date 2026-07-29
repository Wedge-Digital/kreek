# Tests e2e — phase de renvois

**Priorité : haute**
**Dépend de :** 269, 270
**Spec :** `docs/specs/phases-recrutement-renvois/renvois/07-integration.md` §5
**Fichiers :** `tests/e2e/test_dismissals_phase.py` (nouveau), `tests/impact-map.toml`

## Problème

Couverture e2e obligatoire (règle du CLAUDE.md). Et deux propriétés de cette page ne
sont vérifiables **qu'à ce niveau** : le plancher des 11 éligibles vu de l'interface,
et l'absence totale de mouvement de trésorerie de bout en bout.

## Action

### Les onze scénarios

| # | Scénario |
|---|---|
| 1 | « Gérer les renvois → » ouvre la page ; l'effectif est listé avec SPP, valeur et disponibilité |
| 2 | Marquer un joueur : ligne barrée, bouton « Annuler », panier incrémenté |
| 3 | Annuler **depuis la ligne** et **depuis le panier** : les deux chemins fonctionnent |
| 4 | **12 éligibles → un renvoi passe ; à 11, tous les disponibles affichent « Minimum 11 »** |
| 5 | À 11 éligibles, un joueur **absent** reste renvoyable |
| 6 | Valider : les joueurs disparaissent de l'effectif, la trésorerie est **inchangée** |
| 7 | Le grand livre de trésorerie ne gagne **aucune ligne** |
| 8 | Après validation, l'équipe est prête à jouer et la valeur d'équipe **exclut les renvoyés** |
| 9 | Le numéro de maillot libéré est réattribué au recrutement de la séquence suivante |
| 10 | Le panier survit à un aller-retour sur la fiche équipe |
| 11 | Mobile 390px : panier fixe repliable, avertissement en version courte |

### Les scénarios qui portent le plus

**4 et 5** couvrent le plancher, seule vraie subtilité de la page. Le 5 vérifie la
nuance qui compte : un absent ne compte pas parmi les éligibles, donc le renvoyer
n'entame pas le plancher.

**Le 8 est un test de non-régression sur une course.** Sans le second recalcul de
valeur d'équipe (carte 270), il échoue **de façon intermittente** — c'est exactement ce
qu'on attend d'un test sur une course : bruyant plutôt que silencieux. Ne pas le
neutraliser par une attente arbitraire s'il devient instable : c'est le symptôme, pas
le problème.

**Le 7 vérifie une absence**, ce qui semble faible. C'est pourtant le seul test qui
prouve que « un renvoi ne rembourse rien » tient de bout en bout, du `match` exhaustif
de `treasury_movement()` jusqu'au grand livre en base.

### Carte d'impact

Déclarer `test_dismissals_phase` dans `tests/impact-map.toml`. Mêmes BCs traversés
qu'au recrutement.

## Ce qui n'est pas testé ici

La purge des paniers (carte 257) est déclenchée par les quatre entrées en
`ReadyToPlay` et concerne les deux phases : son test appartient à sa propre carte.

## Checklist

- [ ] Les 11 scénarios passent
- [ ] Le 5 vérifie explicitement le cas du joueur absent
- [ ] Le 7 vérifie l'absence de ligne au grand livre
- [ ] Le 8 ne comporte **aucune attente arbitraire** masquant la course
- [ ] Le 11 teste au viewport 390px avec le chrome mobile
- [ ] `test_dismissals_phase` déclaré dans `impact-map.toml`
- [ ] Suite e2e complète toujours verte
- [ ] `make check-arch` au vert, `make test` au vert
