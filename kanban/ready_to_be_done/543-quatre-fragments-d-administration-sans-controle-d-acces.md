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

## Checklist

- [ ] `AuthSession` + `require_admin_access` sur les quatre handlers
- [ ] Quatre cas dans `test_competition_admin_acces.py`, famille « le droit »
- [ ] La commande d'audit rejouée : plus aucun handler routé sans garde
- [ ] `make lint`, `make check-arch`, `make test`, `make e2e`
