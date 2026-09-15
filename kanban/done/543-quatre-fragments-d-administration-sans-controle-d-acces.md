# Quatre fragments d'administration sans contrôle d'accès

**Priorité : moyenne** — divulgation de vues d'administration à un membre simple
du même espace ; aucune écriture n'est atteignable
**Périmètre :** la couche web du BC `competitions`
**Trouvée par :** la carte 519, en cherchant le patron de garde d'un fragment
**Dépend de :** rien
**Fichiers :** `src/app/competitions/io/web/admin/schedule_widgets.rs`,
`groups_widgets.rs`, `tests/e2e/test_competition_admin_acces.py`

## Le constat

Quatre handlers routés en `GET` n'appellent pas `require_admin_access`, et aucune
couche du routeur ne le fait pour eux :

| Handler | Ce qu'il rend |
|---|---|
| `schedule_widgets::schedule_sidebar_widget` | la barre latérale des journées de la saison |
| `schedule_widgets::schedule_round_detail_widget` | le détail d'une journée et ses rencontres |
| `groups_widgets::group_cards_widget` | la composition des poules |
| `groups_widgets::unassigned_pool_widget` | les équipes non affectées |

Vérifié : `src/app/competitions/router.rs` ne porte ni `layer` ni `route_layer`
sur ces routes.

## Ce que `space_scope` ne comble pas

Son propre en-tête le dit : *« une ressource n'est atteignable que depuis son
espace »*. Il garantit qu'un administrateur de l'espace A ne lit pas une
ressource de l'espace B — **pas** que l'appelant est administrateur de l'espace
qu'il vise.

Et il ajoute : *« les paramètres sans résolveur (`round_id`, `pairing_id`,
`action_id`…) passent : ils sont toujours accompagnés d'un parent qui, lui, est
contrôlé »*. Le parent contrôlé l'est **en appartenance d'espace**, pas en droit
d'administration.

## La portée exacte, sans la dramatiser

Ce n'est **ni** un accès anonyme, **ni** un accès inter-espaces, **ni** une
écriture. C'est un **membre simple de l'espace** qui lit quatre vues
d'administration de la compétition.

Le contenu divulgué est modeste : un calendrier et des poules sont largement
publics dans une ligue. Le défaut n'est pas la valeur de la fuite — c'est
l'incohérence : **les pages d'onglet gardent, leurs fragments non**, et rien ne
signale l'écart.

## Pourquoi la carte 416 ne l'a pas attrapé

Elle a fermé le même trou sur **treize routes de mutation** — `groups_actions.rs`
et `schedule_actions.rs`. Son périmètre était les écritures ; les quatre
fragments de lecture n'y figuraient pas, et rien depuis ne les a regardés.

Sa propre conclusion s'applique mot pour mot : *« ces tests sont le seul filet.
Rien dans le compilateur ne signale un handler qui ne contrôle rien, et le projet
n'a pas de harnais au niveau handler (carte 311). Une régression ne se verrait
qu'ici. »*

## Ce qu'il faut faire

1. `AuthSession` en paramètre des quatre handlers, `require_admin_access` en tête
   de corps — le patron exact de `presences_rounds_widget` et
   `presences_panel_widget`, écrits en 519.
2. Étendre `test_competition_admin_acces.py` aux quatre routes de lecture. Le
   fichier teste déjà treize routes de mutation ; la famille « le droit » y gagne
   quatre cas paramétrés.
3. Vérifier qu'aucun autre fragment n'est dans le même cas. La commande qui a
   trouvé ceux-là, à rejouer une fois la correction faite :

```bash
for f in src/app/competitions/io/web/admin/*.rs src/app/competitions/io/web/admin/settings/*.rs; do
  awk -v F="$(basename $f)" '
    /^pub async fn /{ nom=$4; sub(/\(.*/,"",nom); corps=""; enregistre=1 }
    enregistre { corps = corps $0 "\n" }
    /^}$/ && enregistre {
      if (corps !~ /require_admin_access/) printf "%s::%s\n", F, nom
      enregistre=0 }' "$f"
done
```

Elle remonte aussi les résolveurs d'`admin_scope.rs` et `build_summary_fragment`,
qui **ne sont pas des handlers** : ce sont des aides appelées par des handlers
gardés. Ne pas les « corriger ».

## Ce que cette carte ne fait pas

**Elle n'introduit pas de couche de routeur.** Un `route_layer` sur le sous-arbre
d'administration attraperait tout d'un coup, et c'est séduisant — mais
`require_admin_access` a besoin des trois identifiants de chemin
(`space_id`, `competition_id`, `season_id`) et vérifie que la saison appartient à
la compétition. Le porter en couche demanderait de rejouer cette résolution hors
du handler, donc de la dupliquer. À évaluer un jour, pas dans cette carte.

## La commande d'audit avait cessé de dire vrai

Écrite en carte 519, elle rendait **quatre** lignes. Rejouée aujourd'hui, elle en
rend **vingt-deux dont quatre vraies** — et le bruit vient de deux sources que la
carte n'anticipait qu'à moitié :

| Bruit | Pourquoi |
|---|---|
| `admin_scope::*` (cinq), `build_summary_fragment`, `charger_le_panneau` | **non routées** — atteignables seulement depuis un handler déjà gardé |
| `admin_page`, et les **dix** actions de présence | gardées **par une aide** — `render_admin_page` et `contexte()`, que la commande ne suit pas |

La carte prévenait pour les deux premières ; les dix actions de présence sont
arrivées depuis, avec l'onglet, et `charger_le_panneau` avec elles.

**Une vérification qui rend dix-huit faux positifs n'est plus lue.** C'est
exactement ce que le `CLAUDE.md` dit d'une étape sautée : elle rassure au lieu
d'échouer. L'étape 3 de cette carte, prise au pied de la lettre, aurait consisté à
relire dix-huit lignes de bruit après la correction et à conclure que tout va bien.

## L'axe 19 remplace la commande

`scripts/arch/gardes_administration.py`, bloquant. Il croise les fonctions
d'administration avec **le routeur** — ce qui élimine le premier bruit — et connaît
les **aides gardiennes**, déclarées dans une liste courte : `render_admin_page` et
`contexte`.

Et il se garde lui-même : une aide déclarée qui cesserait d'appeler
`require_admin_access` fait échouer l'axe, au lieu d'ouvrir en silence le trou
qu'il surveille. Sans ce contrôle, la liste des aides serait une porte dérobée —
il suffirait d'y inscrire une fonction qui ne garde rien.

Éprouvé dans les deux sens :

| Sabotage | Ce que l'axe dit |
|---|---|
| une garde retirée d'un handler | `groups_widgets.rs::unassigned_pool_widget ← handler d'administration routé sans contrôle d'accès` |
| `contexte` qui n'appelle plus la garde | `contexte — déclarée gardienne, n'appelle plus require_admin_access` |

## Le test e2e a son jumeau, et il a été éprouvé

L'axe voit que la garde est **écrite** ; le test e2e voit qu'elle **répond**. Les
deux sont nécessaires, et le second a été vérifié en retirant une garde : la suite
a rendu `!! GET schedule/rounds → 200`, précisément le défaut de cette carte.

Un second test montre que les mêmes adresses rendent `200` à un administrateur —
sans lui, une route cassée ou un identifiant faux donnerait aussi « pas 200 » au
membre simple, et le premier test passerait pour de mauvaises raisons.

## Checklist

- [x] `AuthSession` + `require_admin_access` sur les quatre handlers
- [x] Deux tests dans `test_competition_admin_acces.py` : le refus du membre
      simple sur les quatre fragments, et son jumeau qui montre que
      l'administrateur les obtient
- [x] La commande d'audit **remplacée** par l'axe 19 — elle rendait dix-huit faux
      positifs et ne vérifiait plus rien
- [x] L'axe éprouvé dans ses deux modes d'échec, le test e2e dans le sien
- [x] `make lint`, `make check-arch`, `make test`, `make e2e`
