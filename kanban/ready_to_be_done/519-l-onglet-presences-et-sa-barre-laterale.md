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

- [ ] Les douze routes, l'onglet dans `admin-page.html`
- [ ] `presences_tab.rs` — page entière ou fragment selon `veut_la_page_entiere`
- [ ] `get_presence_rounds` + son gabarit
- [ ] `PresenceRoundItemVm::from_domain`, `etat` et `resume` venus du domaine
- [ ] La feuille CSS, inscrite dans le bundle au bon rang
- [ ] `require_admin_access` sur les trois handlers
- [ ] `make lint`, `make check-arch`, `make test`
