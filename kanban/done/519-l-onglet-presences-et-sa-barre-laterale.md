# L'onglet Présences et sa barre latérale

**Priorité : haute — la coquille que les deux cartes suivantes remplissent**
**Épic :** E16 — Sondage de présence
**Dépend de :** 510, 511
**Fichiers :** `src/app/competitions/routes.rs`,
`src/app/competitions/io/web/admin/presences_tab.rs`,
`presences_widgets.rs`,
`src/app/competitions/io/web/templates/admin/presences.html`,
`.../admin/widgets/presences-rounds.html`,
`.../admin-page.html`,
`assets/static/css/pages/competition-admin-presences.css`,
`src/web/css_bundle.rs`

## L'objectif

L'onglet existe, il s'ouvre, et sa barre latérale liste les journées avec l'état
de leur campagne. Le panneau de droite est un conteneur vide que la 520 remplit.

## Les routes

Douze constantes dans `routes.rs`, sur le patron des dix-huit de `schedule` :

```
/app/{space_id}/competitions/{competition_id}/{season_id}/admin/presences
                                                          .../presences/rounds
                                                          .../presences/panel
                                                          .../presences/launch
                                                          .../presences/answer
                                                          .../presences/remind
                                                          .../presences/close
                                                          .../presences/reopen
                                                          .../presences/draw
                                                          .../presences/confirm-draw
                                                          .../presences/undo-draw
                                                          .../presences/repair
```

L'onglet dans `admin-page.html`, entre Calendrier et Paramètres, avec
`active_tab == "presences"` et son `hx-push-url`.

## La page hôte est un assemblage

Deux conteneurs `hx-get`, quasi zéro JS — comme `schedule.html`. La géométrie est
celle du Calendrier, journées à gauche et panneau à droite, **pour qu'on ne se
réoriente pas d'un onglet à l'autre**.

L'astuce à reprendre : `document.body.dataset.activeRoundId`. Le panneau écoute
`roundSelected`, qui porte la journée, et `presenceChanged`, qui ne porte rien.
Sans mémoire de la journée courante, toute mutation rechargerait un panneau vide.

```html
hx-vals='js:{"round_id": (event && event.detail && event.detail.round_id)
             || document.body.dataset.activeRoundId || ""}'
```

**`roundSelected` est réutilisé, pas dupliqué.** L'événement existe déjà, émis
par la barre latérale du Calendrier ; les deux onglets ne coexistent jamais.
Bénéfice, et raison du choix : `activeRoundId` survit au changement d'onglet,
donc l'organisateur qui regarde la journée 3 au Calendrier retrouve la journée 3
en Présences.

## La barre latérale

Une seule requête pour toute la saison (`list_summaries`). `PresenceRoundItemVm`
porte `etat` et `resume` — **calculés par le domaine, pas par le gabarit** : le
gabarit choisit une pastille et imprime une phrase, il ne décide pas laquelle.
`etat` vient de `statut_de(...)`, la fonction libre posée par la carte 511.

## La garde, sur les fragments aussi

Chaque handler, **fragment compris**, appelle `require_admin_access` puis
`journee_de_la_saison` — sans quoi le chemin htmx du changement d'onglet
contournerait le contrôle d'accès, et `space_scope` n'a pas de résolveur pour
`round_id`, qui passe donc librement (carte 416).

## Le CSS

`pages/competition-admin-presences.css`, portée `.competition-admin-presences`,
**inscrite dans `src/web/css_bundle.rs`** — l'axe 14 de `check-arch` refuse toute
feuille absente du bundle, et aucun gabarit ne porte de `<link>` : la règle a été
inversée par la carte 342, et c'est ce qui a supprimé le clignotement.

Aucun `style="..."`, les tokens `--p0` à `--p5`, le breakpoint `768px`.
`hx-disinherit="*"` sur la racine du widget.

## Checklist

- [x] **Trois** routes et non douze — chaque carte déclare celles qu'elle sert,
      responsabilité écrite dans les cartes 520, 521 et 522
- [x] L'onglet dans `admin-page.html`, et `"presences"` dans le `match active_tab`
- [x] `presences_tab.rs` — page entière ou fragment selon `veut_la_page_entiere`
- [x] `presences_rounds_widget` + son gabarit, une requête pour toute la saison
- [x] `PresenceRoundItemVm`, `etat` venu de `statut_de(...)`
- [x] La feuille CSS, inscrite dans le bundle entre `groups` et `schedule`
- [x] `require_admin_access` sur les trois handlers, fragments compris
- [x] 8 tests unitaires · `test_presences_tab.py` — 7 tests, entrée dans la carte
      d'impact dans le même commit
- [x] `make lint`, `make check-arch`, `make test` — 1863/1863 · `make e2e` — 375/375

## Ce que la réalisation a tranché

**`etat` n'a pas de `defection`.** Le désaccord se calcule par `desaccord`, qui
exige les appariements de la journée — que `SurveySummaryDto` ne porte pas. La
barre latérale dit « appariée » ; le panneau dira « défection à traiter » (520,
521). L'afficher ici coûterait une jointure par journée pour une nuance que
l'écran voisin porte déjà.

**Aucun nombre inventé.** La maquette annonce « 4 matchs créés » pour une journée
appariée ; le DTO ne compte pas les appariements, et le déduire de `presents / 2`
serait faux dès qu'une équipe est exemptée. Le résumé dit « Journée appariée ».
L'afficher demandera que le DTO compte — c'est noté dans le code.

**Une échéance illisible est journalisée, pas escamotée.** Un
`unwrap_or_default` silencieux ferait disparaître une campagne de la barre
latérale sans une ligne de journal. Elle est signalée en `error!` avec son
`round_id`, et la journée reste affichée. Test dédié.

**L'horloge est lue au bord IO.** La convention « `today` est une entrée » vise
les use cases, qu'elle rend testables ; un rendu doit bien la lire quelque part.

## Deux erreurs corrigées en route

Le `hx-target` de l'onglet visait `#admin-tab-content`, un identifiant inventé,
là où les quatre onglets voisins ciblent `#admin-content`. Le compilateur ne le
voit pas ; seul le test htmx l'attrape.

L'appel de fixture e2e passait un nom en troisième position, là où
`build_full_competition` attend `num_teams: int`. Les sept tests ont échoué en
erreur de fixture — trouvée en quatre secondes en lançant le fichier seul, contre
huit minutes de suite complète.

## Un trou trouvé à côté — carte 543

En cherchant le patron de garde d'un fragment : `schedule_sidebar_widget`,
`schedule_round_detail_widget`, `group_cards_widget` et `unassigned_pool_widget`
n'appellent pas `require_admin_access`, et aucune couche du routeur ne le fait
pour eux. La carte 416 avait fermé le même trou sur les treize routes de
**mutation** ; les fragments de lecture n'y figuraient pas.
