# Les tests E2E de l'onglet Trésorerie

**Épic :** E06 — La fiche d'équipe complétée · **Ordre :** 3 · **Dépend de :** 436
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

- [x] Les six scénarios
- [x] `cliquer_quand_cable` sur chaque clic d'onglet
- [x] Aucun `sleep`
- [x] `make e2e` vert, serveur de développement lancé par l'utilisateur

---

# Ce que la réalisation a appris

## Un seul montage, et trois contraintes qui le fixent

Le relevé n'a d'histoire à raconter qu'après un match publié. Trois choix,
chacun forcé par les données :

**La paire jouée doit être une paire du calendrier.** Le contexte de match vient
de `competition_match_display_proj`, que le listener n'alimente que si la paire
correspond à un `competition_match_day_pairings`. Une paire arbitraire — la voie
la plus simple, et celle que d'autres modules empruntent — donnerait un relevé
sans titre de journée, et le premier test passerait sans rien prouver de ce qui
compte.

**Les coups de pouce sont achetés par le camp le plus fort.** C'est la seule
ligne du grand livre qui porte un identifiant de rapport, donc la seule qui
puisse ouvrir une période. Et l'underdog paie sur sa petite monnaie, qui ne sort
d'aucune caisse et n'écrit aucune ligne : acheter pour lui ne produirait rien du
tout. Les quatre équipes partagent le même roster, leurs valeurs sont donc
égales, et la petite monnaie du premier acheteur est nulle.

**Une équipe de la seconde paire n'est jamais jouée** et sert l'état vide, sans
seconde compétition à construire.

## L'assertion qui manquait au scénario de l'aller-retour

La carte décrit `test_l_onglet_joueurs_reste_accessible_apres_un_aller_retour`
comme « la régression que le découpage en onglets peut créer ». Le test livré
vérifie **quel onglet est souligné**, et pas seulement que l'effectif revient.

Ce n'est pas un zèle : c'est exactement le défaut trouvé à l'écran sur la carte
436 — le contenu basculait, le soulignement non, et l'effectif s'affichait sous
un onglet « Trésorerie » actif. Un test qui ne regarderait que le tableau des
joueurs serait passé au vert sur cette application-là. Falsifié : sortir le
bandeau de la cible du swap fait bien rougir ce test.

## Falsification

| Mutation | Constaté |
|---|---|
| Le bandeau ressort de la cible du swap (défaut de la 436) | `…apres_un_aller_retour` rouge |
| Plus aucun titre de période | `…s_ouvre_et_affiche_le_releve` rouge |
| La page complète renvoie le fragment nu | `…se_charge_directement` rouge |
| Le solde du relevé retombe sur la dotation | `…egale_celui_de_l_en_tete` rouge |
| L'état vide ne se déclenche jamais | `…affiche_le_bloc_sans_mouvement` rouge |
| La jointure vers l'effectif rompue | `…le_recrutement_qui_vient_d_etre_fait` rouge |

## Deux sélecteurs qui désignaient autre chose que ce qu'on croyait

`.player-table` désigne **deux** tableaux — l'effectif et le staff partagent la
classe. `#players-widget` n'est que le conteneur d'attente : le widget arrive en
`hx-swap="outerHTML"` et l'`id` disparaît avec lui. C'est `.players-widget`, la
racine que le BC `players` rend, qui désigne l'effectif.

## Ce que ce travail a révélé au passage, et qui le dépasse

**L'axe 8 de `check-arch` ne vérifie rien sur cette machine.** Il importe
`tomllib`, absent avant Python 3.11 ; le `python3` du système est en 3.9.6. Son
`2>/dev/null || true` avale l'échec de l'import, la sortie est vide, et
l'absence d'anomalie est lue comme un succès.

Exécuté avec un interpréteur moderne, il remonte **cinq fichiers de test sans
entrée dans `impact-map.toml`**, tous ajoutés entre le 25 et le 29 août.

La CI ne le rattrape pas : elle ne se déclenche que sur `main` et sur les pull
requests vers `main`, et ce travail vit sur `demo`.

C'est mot pour mot ce que le `CLAUDE.md` décrit à propos de l'audit — « une
étape sautée doit échouer, pas rassurer ». Hors périmètre de cette carte ; à
traiter à part.
