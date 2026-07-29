# Tests e2e — phase de recrutement

**Priorité : haute**
**Dépend de :** 264, 265
**Spec :** `docs/specs/phases-recrutement-renvois/recrutement/07-integration.md` §5
**Fichiers :** `tests/e2e/test_recruitment_phase.py` (nouveau), `tests/impact-map.toml`

## Problème

La règle de couverture du CLAUDE.md impose un test e2e par fonctionnalité livrée : le
test unitaire vérifie la logique, l'e2e vérifie que le rendu HTMX/Alpine fonctionne
réellement dans un navigateur.

Ici l'enjeu est particulier — c'est le seul niveau où l'on peut vérifier que le
**panier serveur** tient sa promesse.

## Action

### Les onze scénarios

| # | Scénario |
|---|---|
| 1 | La bannière « Recruter → » ouvre la page ; le catalogue liste les postes du roster avec leurs prix |
| 2 | Ajouter un joueur : la ligne apparaît au panier, le reste de trésorerie diminue, le quota affiche `+1` |
| 3 | Retirer la ligne : panier vide, trésorerie et quota revenus à l'état initial |
| 4 | **Trésorerie insuffisante** : le bouton affiche « Trésorerie » et **rien n'est débité** |
| 5 | Quota de poste atteint : le bouton affiche « Quota atteint » |
| 6 | Roster sans apothicaire : la ligne affiche le motif, le bouton est inactif |
| 7 | Relance affichée au double du prix de base, prix de base rappelé |
| 8 | Valider : la trésorerie est débitée **du total**, les joueurs existent, l'équipe passe en phase de renvois |
| 9 | **Quitter la page sans valider, y revenir : le panier est toujours là** |
| 10 | Le grand livre contient une ligne par achat, avec le solde après |
| 11 | Mobile 390px : panier fixe repliable, `×` atteignables |

### Les deux scénarios qui comptent le plus

**Le 9** vérifie ce que la décision D1 a acheté. Sans lui, on aurait payé une table de
panier pour rien — c'est le seul test qui distingue un panier serveur d'un panier
client.

**Le 4** vérifie qu'aucun débit n'a lieu avant validation. C'est la propriété qui rend
le panier sûr : un clic malheureux ne coûte rien tant que la phase n'est pas
validée.

### Le scénario 10 lit la base

Il n'existe aucun écran d'historique de trésorerie — une page de trésorerie n'est pas
au périmètre de cette feature. Le test passe donc par `db_helpers.py`.

### Carte d'impact

Déclarer `test_recruitment_phase` dans `tests/impact-map.toml`. BCs traversés :
`teams`, `players`, `references`, plus `team_creation`, `competitions`, `spaces` et
`ranking` en fixture via `build_full_competition()`.

## Checklist

- [ ] Les 11 scénarios passent
- [ ] Le 9 échoue si le panier redevient client — c'est son rôle
- [ ] Le 4 vérifie l'absence de débit, pas seulement le message
- [ ] Le 11 teste au viewport 390px avec le chrome mobile
- [ ] `test_recruitment_phase` déclaré dans `impact-map.toml` avec ses BCs
- [ ] Suite e2e complète toujours verte
- [ ] `make check-arch` au vert, `make test` au vert
