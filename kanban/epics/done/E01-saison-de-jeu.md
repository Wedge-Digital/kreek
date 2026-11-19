# E01 — Saison de jeu : le cycle de vie d'une équipe

**État :** 10 cartes · 10 faites

## La fonction

Une équipe ne se crée pas puis s'immobilise : elle s'inscrit à une compétition,
joue, encaisse ses recettes, améliore ses joueurs, recrute, renvoie, met des
joueurs en retraite temporaire, paie ses erreurs coûteuses, et repart pour une
journée — jusqu'au repos hors-saison. C'est la boucle de jeu du produit.

L'épic couvre cette boucle de bout en bout : l'agrégat `Team` et ses
transitions de phase, la projection qui les rend lisibles sans rejouer l'event
store, et les écrans de chaque phase.

## Les cartes

| # | Intitulé | Apport | Vérifié dans |
|---|---|---|---|
| 42 | Table de projection + requêtes de liste | l'infrastructure de lecture de tout le reste | 3 migrations `team_projection`, repository câblé |
| 32 | Inscription dans une compétition | « Inscrite / Prête à jouer » | `enrollment_actions.rs`, `pending_enrollment_widget.rs` |
| 49 | Règles d'accession et inscription d'équipe | le versant `competitions` de l'inscription | `enrollments_tab.rs`, `requires_validation` |
| 35 | Listener `MatchPlayed` → `PostMatchSequenceStarted` | l'entrée dans la séquence d'après-match | `match_report_published_listener.rs` |
| 36 | Phase d'amélioration des joueurs | dépense de SPP | `domain/team.rs`, projection |
| 37 | Phase de recrutement | achat joueurs et staff | `validate_recruitment_phase_use_case.rs` |
| 38 | Phase de renvois | `validate_dismissals_phase_use_case.rs`, `dismiss_staff.rs`, `dismiss_team.rs` | |
| 39 | Phase de retraite temporaire | | `domain/team.rs`, `my_teams_widget.rs` |
| 40 | Erreur coûteuse + retour « Prête à jouer » | la fermeture de la boucle | `domain/treasury.rs`, `phase_basket_purge_listener.rs` |
| 43 | Repos hors-saison | | `phase_basket_repository.rs`, `team_detail.rs` |

Les dix cartes ont été livrées sans jamais être déplacées en `done/` — elles
l'ont été d'un bloc à la création de cette épic, après vérification une par une
dans le code (colonne de droite).

**Deux d'entre elles ont bougé depuis, et ce tableau ne le disait pas :**

| Carte | Ce qui s'est passé |
|---|---|
| `39` — retraite temporaire | **revenue en `to_be_refined/`** : la vérification l'avait comptée faite à tort |
| `40` — erreur coûteuse | **devenue l'épic E13**, close depuis ; la carte est en `cancelled/` |

Le commit `577fda8` a fait les deux mouvements sans reprendre cette épic. La
clôture de E01 tient malgré la 39 : son « Terminé quand » se constate sur le
tour complet d'une journée, et la retraite temporaire n'en fait pas partie.

## Ce que l'épic ne couvre pas

Trois cartes en ont été **sorties** parce que la vérification a montré qu'elles
ne sont pas faites :

| Carte | Constat |
|---|---|
| `48-team-treasury-tab` | l'onglet « Trésorerie » existe en `teams-team-detail.html:149` mais c'est un `<div class="tab">` **inerte** : ni `hx-get`, ni handler `treasury_tab`. Passée en **E06** |
| `45-team-modification` | aucune trace dans le code. Sans épic |
| `46-team-customisation-admin` | aucune trace d'override admin d'état. Sans épic |

Ne couvre pas non plus le déroulé d'un match lui-même — c'est le BC
`match_report`, qui alimente cette boucle par un app event et vit sa vie à
part.

## Terminé quand

Une équipe traverse une journée complète — match joué, recettes encaissées,
SPP dépensés, recrutement, renvois, retraite temporaire, erreur coûteuse — et
se retrouve « Prête à jouer » pour la journée suivante, sans intervention
manuelle en base.

**Constaté :** oui. La séquence est en production et exercée par la suite e2e.
