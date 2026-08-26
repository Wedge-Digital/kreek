# Les tests E2E de l'onglet Trésorerie

**Ordre :** 3 · **Dépend de :** 436
**Conception :** `docs/specs/tresorerie-equipe/onglet-tresorerie/07-integration.md`

## Objectif

Prouver dans un navigateur ce qu'aucun test unitaire ne voit : le câblage des
onglets, qui n'existait pas, et la concordance des deux chemins vers le solde.

Fichier : `tests/e2e/test_team_treasury_tab.py`.

## Les scénarios

| Test | Ce qu'il prouve |
|---|---|
| `test_l_onglet_tresorerie_s_ouvre_et_affiche_le_releve` | le câblage des onglets |
| `test_l_url_de_tresorerie_se_charge_directement` | `hx-push-url` et le rendu de la page complète — un lien collé doit marcher |
| `test_le_solde_du_releve_egale_celui_de_l_en_tete` | **le test qui compte** |
| `test_une_equipe_neuve_affiche_le_bloc_sans_mouvement` | l'état vide, dotation comprise |
| `test_le_releve_montre_le_recrutement_qui_vient_d_etre_fait` | la jointure vers l'événement, de bout en bout |
| `test_l_onglet_joueurs_reste_accessible_apres_un_aller_retour` | la régression que le découpage en onglets peut créer |

## Celui qui vaut le prix de la suite

**`test_le_solde_du_releve_egale_celui_de_l_en_tete`.**

L'en-tête affiche `treasury_kpo`, lu depuis l'agrégat. Le relevé affiche
`balance_after_kpo`, lu depuis le grand livre. **Ce sont deux chemins vers la
même vérité**, et c'est le seul endroit de l'application où ils sont côte à côte
à l'écran.

Une divergence signifierait que le grand livre a décroché de l'agrégat — ce que
la transaction commune de l'append est censée empêcher. Ce test devient donc le
premier contrôle continu de cette garantie, et c'est un bénéfice qui dépasse
l'écran qu'il vérifie.

## Le piège de la fenêtre non câblée

Le contenu de l'onglet arrive par `hx-get`. Le second clic — retour sur
« Joueurs & Staff » — vise du contenu fraîchement injecté, **exactement la
fenêtre où un élément est peint, cliquable et inerte**.

```python
from htmx_helpers import cliquer_quand_cable
cliquer_quand_cable(page, ".tab:has-text('Trésorerie')")
```

**Pas de `sleep`.** Une durée fixe n'a aucune marge sur une machine chargée —
c'est exactement là que la suite échouait — tout en coûtant son délai aux
milliers d'appels où tout est déjà prêt.

## Ce que les tests ne couvrent pas

- **Les deux refus de cohérence** — dotation absente, motif inconnu — ne se
  provoquent pas depuis un navigateur : ils demandent une base incohérente. La
  carte 435 les couvre unitairement.
- **Les lignes de coups de pouce d'un match manuel**, qui n'ont ni journée ni
  adversaire tant que la **carte 427** n'est pas livrée. Ce n'est pas un défaut
  de cet écran, et le tester figerait un comportement transitoire.

## Checklist

- [ ] Les six scénarios
- [ ] `cliquer_quand_cable` sur chaque clic d'onglet
- [ ] Aucun `sleep`
- [ ] `make e2e` vert, serveur de développement lancé par l'utilisateur
