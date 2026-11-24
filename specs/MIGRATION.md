# Migration des specs vers le kanban de la spec factory

Le 2026-11-19. Les cartes de `kanban/<état>/` et les épics de `kanban/epics/<état>/` ont été rangées ici
(`git mv`, historique conservé : `git log --follow`). Une épic `E<nn>` est devenue l'initiative
`K<nn>` ; son fichier est devenu son `initiative.md`. Les dossiers de conception restent dans
`docs/specs/<fonctionnalité>/`, et chaque initiative qui en découle y renvoie.

| Initiative | Colonne | Origine | Tickets |
|---|---|---|---|
| [K1](7-livree/K1-saison-de-jeu/initiative.md) | `7-livree` | épic E01 | 8 |
| [K2](7-livree/K2-notifications-email/initiative.md) | `7-livree` | épic E02 | 15 |
| [K3](7-livree/K3-front-ni-saut-ni-clignotement/initiative.md) | `7-livree` | épic E03 | 4 |
| [K4](1-backlog/01-K4-verrous-architecturaux/initiative.md) | `1-backlog` | épic E04 | 6 |
| [K5](1-backlog/02-K5-couverture-e2e/initiative.md) | `1-backlog` | épic E05 | 4 |
| [K6](1-backlog/03-K6-fiche-equipe-completee/initiative.md) | `1-backlog` | épic E06 | 1 |
| [K7](5-tickets/K7-entrees-utilisateur-et-identite/initiative.md) | `5-tickets` | épic E07 | 1 |
| [K8](7-livree/K8-mode-customisation/initiative.md) | `7-livree` | épic E08 | 1 |
| [K9](1-backlog/04-K9-bc-news/initiative.md) | `1-backlog` | épic E09 | 2 |
| [K10](1-backlog/05-K10-referentiels-editables/initiative.md) | `1-backlog` | épic E10 | 20 |
| [K11](7-livree/K11-journal-de-production/initiative.md) | `7-livree` | épic E11 | 9 |
| [K12](7-livree/K12-administrer-les-membres-d-un-espace/initiative.md) | `7-livree` | épic E12 | 21 |
| [K13](7-livree/K13-gestion-des-erreurs-couteuses/initiative.md) | `7-livree` | épic E13 | 4 |
| [K14](7-livree/K14-modifier-une-competition-en-cours/initiative.md) | `7-livree` | épic E14 | 10 |
| [K15](7-livree/K15-recruter-un-journalier/initiative.md) | `7-livree` | épic E15 | 10 |
| [K16](7-livree/K16-imprimer-la-feuille-d-equipe/initiative.md) | `7-livree` | épic E16 | 17 |
| [K18](7-livree/K18-appariement-des-presents/initiative.md) | `7-livree` | épic E18 | 8 |
| [K19](7-livree/K19-saisie-du-compte-rendu-droits-et-robustesse/initiative.md) | `7-livree` | épic E19 | 5 |
| [K20](7-livree/K20-coups-de-pouce-panier-et-annulation/initiative.md) | `7-livree` | épic E20 | 5 |
| [K21](7-livree/K21-mode-de-saisie-et-apres-match-en-sequence/initiative.md) | `7-livree` | épic E21 | 9 |
| [K22](6-prete/K22-mode-chacun-son-equipe/initiative.md) | `6-prete` | épic E22 | 5 |
| [K23](7-livree/K23-temps-reel-connexions-et-notifications/initiative.md) | `7-livree` | épic E23 | 6 |
| [K24](1-backlog/06-K24-navigation-mobile/initiative.md) | `1-backlog` | épic E24 | 0 |
| [K25](1-backlog/07-K25-retrouver-ses-matchs/initiative.md) | `1-backlog` | épic E25 | 0 |
| [K26](1-backlog/08-K26-equipes-sur-mobile/initiative.md) | `1-backlog` | épic E26 | 0 |
| [K27](7-livree/K27-accueil-derniers-resultats/initiative.md) | `7-livree` | `docs/specs/accueil-derniers-resultats/` | 7 |
| [K28](7-livree/K28-ajustement-tresorerie/initiative.md) | `7-livree` | `docs/specs/ajustement-tresorerie/` | 4 |
| [K29](7-livree/K29-competition-admin/initiative.md) | `7-livree` | `docs/specs/competition-admin/` | 4 |
| [K30](7-livree/K30-competition-matchs/initiative.md) | `7-livree` | `docs/specs/competition-matchs/` | 9 |
| [K31](7-livree/K31-haine/initiative.md) | `7-livree` | `docs/specs/haine/` | 6 |
| [K32](7-livree/K32-match-report/initiative.md) | `7-livree` | `docs/specs/match-report/` | 51 |
| [K33](7-livree/K33-match-report-correction/initiative.md) | `7-livree` | `docs/specs/match-report-correction/` | 12 |
| [K34](7-livree/K34-my-teams/initiative.md) | `7-livree` | `docs/specs/my-teams/` | 5 |
| [K35](7-livree/K35-phases-manuelles/initiative.md) | `7-livree` | `docs/specs/phases-manuelles/` | 5 |
| [K36](7-livree/K36-phases-recrutement-renvois/initiative.md) | `7-livree` | `docs/specs/phases-recrutement-renvois/` | 17 |
| [K37](7-livree/K37-player-customisation/initiative.md) | `7-livree` | `docs/specs/player-customisation/` | 8 |
| [K38](7-livree/K38-player-edition/initiative.md) | `7-livree` | `docs/specs/player-edition/` | 6 |
| [K39](7-livree/K39-player-match-impact/initiative.md) | `7-livree` | `docs/specs/player-match-impact/` | 17 |
| [K40](7-livree/K40-player-spp-spending/initiative.md) | `7-livree` | `docs/specs/player-spp-spending/` | 9 |
| [K41](7-livree/K41-points-classement-manuels/initiative.md) | `7-livree` | `docs/specs/points-classement-manuels/` | 5 |
| [K42](7-livree/K42-post-match-team-effects/initiative.md) | `7-livree` | `docs/specs/post-match-team-effects/` | 1 |
| [K43](7-livree/K43-ranking/initiative.md) | `7-livree` | `docs/specs/ranking/` | 24 |
| [K44](7-livree/K44-ranking-bonus-points/initiative.md) | `7-livree` | `docs/specs/ranking-bonus-points/` | 8 |
| [K45](6-prete/K45-statistiques-de-competition/initiative.md) | `6-prete` | `docs/specs/statistiques-de-competition/` | 2 |
| [K46](7-livree/K46-team-state-management/initiative.md) | `7-livree` | `docs/specs/team-state-management/` | 5 |
| [K47](1-backlog/09-K47-decisions-de-regle-en-attente/initiative.md) | `1-backlog` | groupe du README | 2 |
| [K48](1-backlog/10-K48-actions-sur-une-equipe-hors-cycle/initiative.md) | `1-backlog` | groupe du README | 1 |
| [K49](6-prete/K49-dette-technique-diffuse/initiative.md) | `6-prete` | groupe du README | 5 |
| [K50](7-livree/K50-un-seul-service-pour-savoir-qui-est-admin/initiative.md) | `7-livree` | groupe du README | 4 |
| [K51](7-livree/K51-todo-production/initiative.md) | `7-livree` | carte isolée | 1 |
| [K52](7-livree/K52-todo-fake-test/initiative.md) | `7-livree` | carte isolée | 1 |
| [K53](7-livree/K53-unwrap-or-default-render/initiative.md) | `7-livree` | carte isolée | 1 |
| [K54](7-livree/K54-erreurs-db-silencieuses/initiative.md) | `7-livree` | carte isolée | 1 |
| [K55](7-livree/K55-unwrap-handlers/initiative.md) | `7-livree` | carte isolée | 1 |
| [K56](7-livree/K56-groupement-on2/initiative.md) | `7-livree` | carte isolée | 1 |
| [K57](7-livree/K57-supprimer-bckp/initiative.md) | `7-livree` | carte isolée | 1 |
| [K58](7-livree/K58-css-chargement-global/initiative.md) | `7-livree` | carte isolée | 1 |
| [K59](7-livree/K59-roster-selection-widget/initiative.md) | `7-livree` | série par dépendances | 17 |
| [K60](7-livree/K60-team-dismissal-admin/initiative.md) | `7-livree` | carte isolée | 1 |
| [K61](7-livree/K61-ref-skill-picker-widget/initiative.md) | `7-livree` | série par dépendances | 16 |
| [K62](7-livree/K62-tc-league-selection/initiative.md) | `7-livree` | carte isolée | 1 |
| [K63](7-livree/K63-players-bc-structure-aggregate/initiative.md) | `7-livree` | série par dépendances | 5 |
| [K64](7-livree/K64-cli-infrastructure/initiative.md) | `7-livree` | série par dépendances | 2 |
| [K65](7-livree/K65-build-team-validations/initiative.md) | `7-livree` | carte isolée | 1 |
| [K66](7-livree/K66-mr-step2-page-post/initiative.md) | `7-livree` | carte isolée | 1 |
| [K67](7-livree/K67-mr-step2-e2e-tests/initiative.md) | `7-livree` | carte isolée | 1 |
| [K68](7-livree/K68-mr-published-competitions-listener/initiative.md) | `7-livree` | carte isolée | 1 |
| [K69](7-livree/K69-arch-false-positives-and-axe5-scope/initiative.md) | `7-livree` | série par dépendances | 2 |
| [K70](7-livree/K70-arch-axe4-hardcoded-routes/initiative.md) | `7-livree` | carte isolée | 1 |
| [K71](7-livree/K71-arch-axe3-players-sovereignty/initiative.md) | `7-livree` | carte isolée | 1 |
| [K72](7-livree/K72-arch-axe3-competitions-sovereignty/initiative.md) | `7-livree` | carte isolée | 1 |
| [K73](7-livree/K73-arch-axe3-teams-sovereignty/initiative.md) | `7-livree` | carte isolée | 1 |
| [K74](7-livree/K74-arch-axe3-team-creation-sovereignty/initiative.md) | `7-livree` | carte isolée | 1 |
| [K75](7-livree/K75-arch-axe3-references-rostername-shared-kernel/initiative.md) | `7-livree` | carte isolée | 1 |
| [K76](7-livree/K76-check-arch-bcs-manquants/initiative.md) | `7-livree` | carte isolée | 1 |
| [K77](7-livree/K77-sp-annulation-rapport-engage/initiative.md) | `7-livree` | série par dépendances | 3 |
| [K78](7-livree/K78-sp-find-id-by-pairing-cancelled/initiative.md) | `7-livree` | carte isolée | 1 |
| [K79](7-livree/K79-extraction-auth-spaces/initiative.md) | `7-livree` | série par dépendances | 7 |
| [K80](7-livree/K80-valeur-joueur-unite-kpo/initiative.md) | `7-livree` | série par dépendances | 4 |
| [K81](7-livree/K81-comptage-joueurs-disponibles/initiative.md) | `7-livree` | carte isolée | 1 |
| [K82](7-livree/K82-formatage-rustfmt/initiative.md) | `7-livree` | série par dépendances | 2 |
| [K83](7-livree/K83-bareme-spp-brawlin-brutes/initiative.md) | `7-livree` | série par dépendances | 2 |
| [K84](7-livree/K84-teams-detail-retry/initiative.md) | `7-livree` | carte isolée | 1 |
| [K85](7-livree/K85-widget-tester-bypass-space-member-port/initiative.md) | `7-livree` | carte isolée | 1 |
| [K86](7-livree/K86-check-arch-axe3-chainage-multiligne/initiative.md) | `7-livree` | carte isolée | 1 |
| [K87](7-livree/K87-mobile-tabbar-hauteur-token/initiative.md) | `7-livree` | carte isolée | 1 |
| [K88](7-livree/K88-auth-coach-name-case-insensitive/initiative.md) | `7-livree` | carte isolée | 1 |
| [K89](7-livree/K89-env-demo-renommage-remote/initiative.md) | `7-livree` | carte isolée | 1 |
| [K90](7-livree/K90-harnais-test-handler/initiative.md) | `7-livree` | carte isolée | 1 |
| [K91](7-livree/K91-tv-suit-la-customisation-de-prix/initiative.md) | `7-livree` | carte isolée | 1 |
| [K92](7-livree/K92-journal-affiche-les-customisations/initiative.md) | `7-livree` | carte isolée | 1 |
| [K93](7-livree/K93-players-appartenance-espace/initiative.md) | `7-livree` | carte isolée | 1 |
| [K94](7-livree/K94-transaction-fantome-flakiness-e2e/initiative.md) | `7-livree` | carte isolée | 1 |
| [K95](7-livree/K95-competitions-appartenance-espace/initiative.md) | `7-livree` | série par dépendances | 6 |
| [K96](7-livree/K96-ts-team-deux-tailles-de-police/initiative.md) | `7-livree` | série par dépendances | 2 |
| [K97](7-livree/K97-migrations-de-donnees-au-demarrage/initiative.md) | `7-livree` | série par dépendances | 3 |
| [K98](7-livree/K98-modifier-l-effectif-reserve-a-qui-de-droit/initiative.md) | `7-livree` | carte isolée | 1 |
| [K99](7-livree/K99-les-mots-clefs-et-les-traits-s-affichent/initiative.md) | `7-livree` | carte isolée | 1 |
| [K100](7-livree/K100-un-mercenaire-trois-quarts-disparait-du-rapport/initiative.md) | `7-livree` | carte isolée | 1 |
| [K101](7-livree/K101-inscription-dans-une-competition-non-finalisee/initiative.md) | `7-livree` | carte isolée | 1 |
| [K102](7-livree/K102-la-phase-finale-quitte-la-creation-de-competitio/initiative.md) | `7-livree` | carte isolée | 1 |
| [K103](7-livree/K103-retirer-une-customisation-appliquee/initiative.md) | `7-livree` | carte isolée | 1 |
| [K104](7-livree/K104-treize-routes-d-administration-sans-controle/initiative.md) | `7-livree` | carte isolée | 1 |
| [K105](7-livree/K105-un-rapport-manuel-en-cours-n-apparait-nulle-part/initiative.md) | `7-livree` | série par dépendances | 2 |
| [K106](7-livree/K106-test-de-purge-des-paniers-instable/initiative.md) | `7-livree` | carte isolée | 1 |
| [K107](7-livree/K107-url-publique-et-schema/initiative.md) | `7-livree` | carte isolée | 1 |
| [K108](7-livree/K108-deux-tokens-de-gris-indiscernables/initiative.md) | `7-livree` | carte isolée | 1 |
| [K109](7-livree/K109-sous-total-des-joueurs-disponibles/initiative.md) | `7-livree` | carte isolée | 1 |
| [K110](7-livree/K110-tout-membre-d-une-competition-est-commissaire/initiative.md) | `7-livree` | carte isolée | 1 |
| [K111](7-livree/K111-l-axe-8-de-check-arch-ne-verifie-rien/initiative.md) | `7-livree` | carte isolée | 1 |
| [K112](7-livree/K112-bonus-pour-non-temporisation/initiative.md) | `7-livree` | carte isolée | 1 |
| [K113](7-livree/K113-une-competence-gratuite-rencherit-la-suivante/initiative.md) | `7-livree` | carte isolée | 1 |
| [K114](7-livree/K114-le-montage-e2e-compte-les-clics-pas-les-embauche/initiative.md) | `7-livree` | carte isolée | 1 |
| [K115](7-livree/K115-cliquer-une-equipe-ne-fait-rien/initiative.md) | `7-livree` | carte isolée | 1 |
| [K116](7-livree/K116-modifier-une-competition-la-remet-en-constructio/initiative.md) | `7-livree` | carte isolée | 1 |
| [K117](7-livree/K117-un-refus-de-depublication-ne-se-distingue-pas-d/initiative.md) | `7-livree` | carte isolée | 1 |
| [K118](7-livree/K118-du-css-qui-ne-trouve-pas-son-markup/initiative.md) | `7-livree` | carte isolée | 1 |
| [K119](7-livree/K119-un-mort-ne-compte-plus/initiative.md) | `7-livree` | carte isolée | 1 |
| [K120](7-livree/K120-un-joueur-indisponible-se-voit-dans-la-liste/initiative.md) | `7-livree` | carte isolée | 1 |
| [K121](7-livree/K121-les-coachs-sont-deconnectes-trop-souvent/initiative.md) | `7-livree` | série par dépendances | 2 |
| [K122](7-livree/K122-la-liste-affiche-les-spp-gagnes-au-lieu-du-solde/initiative.md) | `7-livree` | carte isolée | 1 |
| [K123](7-livree/K123-un-test-de-journalisation-accusait-le-produit/initiative.md) | `7-livree` | carte isolée | 1 |
| [K124](7-livree/K124-l-etape-2-affiche-deux-choses-fausses/initiative.md) | `7-livree` | carte isolée | 1 |
| [K125](7-livree/K125-l-ecran-annonce-moins-de-journaliers-qu-il-n-en/initiative.md) | `7-livree` | carte isolée | 1 |
| [K126](7-livree/K126-deux-tests-lisent-la-projection-avant-que-le-bus/initiative.md) | `7-livree` | carte isolée | 1 |
| [K127](7-livree/K127-le-menu-du-selecteur-etait-rogne-par-son-panneau/initiative.md) | `7-livree` | carte isolée | 1 |
| [K128](7-livree/K128-la-competence-bonus-de-debut-de-ligue-ampute-le/initiative.md) | `7-livree` | carte isolée | 1 |
| [K129](7-livree/K129-la-correction-etait-jugee-avant-que-les-equipes/initiative.md) | `7-livree` | carte isolée | 1 |
| [K130](7-livree/K130-le-bandeau-n-offre-que-ce-qu-on-a-le-droit-de-fa/initiative.md) | `7-livree` | carte isolée | 1 |
| [K131](7-livree/K131-les-actions-d-equipe-se-gardent-elles-memes/initiative.md) | `7-livree` | carte isolée | 1 |
| [K132](7-livree/K132-evenements-fantomes-et-jokers/initiative.md) | `7-livree` | carte isolée | 1 |
| [K133](7-livree/K133-cuistot-halfling-prix-reduit/initiative.md) | `7-livree` | carte isolée | 1 |
| [K134](7-livree/K134-logo-equipe-methode-domaine/initiative.md) | `7-livree` | série par dépendances | 6 |
| [K135](7-livree/K135-qui-peut-administrer-une-competition-et-qui-le-v/initiative.md) | `7-livree` | carte isolée | 1 |
| [K136](7-livree/K136-interdire-les-matchs-hors-calendrier/initiative.md) | `7-livree` | carte isolée | 1 |
| [K137](7-livree/K137-une-equipe-joue-au-plus-un-match-par-journee/initiative.md) | `7-livree` | série par dépendances | 4 |
| [K138](7-livree/K138-le-toast-que-personne-n-ecoute/initiative.md) | `7-livree` | carte isolée | 1 |
| [K139](7-livree/K139-une-ligne-de-match-mene-toujours-a-son-rapport/initiative.md) | `7-livree` | carte isolée | 1 |
| [K140](7-livree/K140-les-places-d-un-joueur-mort-sont-libres/initiative.md) | `7-livree` | carte isolée | 1 |
| [K141](7-livree/K141-le-tarif-reduit-de-chantage-et-corruption/initiative.md) | `7-livree` | série par dépendances | 2 |
| [K142](7-livree/K142-la-page-des-erreurs-couteuses-sans-ses-styles/initiative.md) | `7-livree` | carte isolée | 1 |
| [K143](7-livree/K143-un-conflit-de-version-n-est-pas-une-panne/initiative.md) | `7-livree` | carte isolée | 1 |
| [K144](7-livree/K144-la-phase-figee-part-vide/initiative.md) | `7-livree` | carte isolée | 1 |
| [K145](7-livree/K145-le-widget-d-equipes-recevait-deux-fois-la-saison/initiative.md) | `7-livree` | carte isolée | 1 |
| [K146](7-livree/K146-le-widget-d-identite-d-equipe/initiative.md) | `7-livree` | série par dépendances | 2 |
| [K147](7-livree/K147-les-blessures-persistantes-sur-la-feuille-d-equi/initiative.md) | `7-livree` | série par dépendances | 2 |
| [K148](7-livree/K148-le-test-du-journalier-recrute-est-instable/initiative.md) | `7-livree` | carte isolée | 1 |
| [K149](7-livree/K149-le-deplacement-de-match-echoue-une-fois-sur-sept/initiative.md) | `7-livree` | carte isolée | 1 |
| [K150](7-livree/K150-retirer-des-spp-par-la-customisation/initiative.md) | `7-livree` | carte isolée | 1 |
| [K151](7-livree/K151-test-de-purge-des-paniers-instable/initiative.md) | `7-livree` | carte isolée | 1 |
| [K152](7-livree/K152-url-publique-et-schema/initiative.md) | `7-livree` | carte isolée | 1 |
| [K153](7-livree/K153-le-widget-du-logo-charge-cloudinary-sur-chaque-f/initiative.md) | `7-livree` | carte isolée | 1 |
| [K154](7-livree/K154-la-valeur-d-equipe-oublie-les-recrues-d-un-recru/initiative.md) | `7-livree` | carte isolée | 1 |
| [K155](7-livree/K155-le-test-du-droit-de-deplacer-ne-s-execute-jamais/initiative.md) | `7-livree` | carte isolée | 1 |
| [K156](7-livree/K156-la-suite-e2e-ralentit-avec-sa-base/initiative.md) | `7-livree` | carte isolée | 1 |
| [K157](7-livree/K157-la-creation-de-competition-rend-la-main-avant-sa/initiative.md) | `7-livree` | carte isolée | 1 |
| [K158](7-livree/K158-la-phase-de-renvoi-clique-avant-le-cablage/initiative.md) | `7-livree` | carte isolée | 1 |
| [K159](7-livree/K159-la-correction-ouverte-au-coach-lit-trop-tot/initiative.md) | `7-livree` | carte isolée | 1 |
| [K160](7-livree/K160-le-recalcul-de-la-valeur-d-equipe-se-perd/initiative.md) | `7-livree` | carte isolée | 1 |
| [K161](7-livree/K161-le-recrutement-clique-avant-le-cablage/initiative.md) | `7-livree` | carte isolée | 1 |
| [K162](7-livree/K162-le-brouillon-d-equipe-clique-sous-un-menu-ouvert/initiative.md) | `7-livree` | carte isolée | 1 |
| [K163](6-prete/K163-script-inline-htmx-fragments/initiative.md) | `6-prete` | carte isolée | 1 |
| [K164](1-backlog/11-K164-temporary-retirement-phase/initiative.md) | `1-backlog` | carte isolée | 1 |
| [K165](1-backlog/12-K165-jersey-numbers-at-submission/initiative.md) | `1-backlog` | carte isolée | 1 |
| [K166](1-backlog/13-K166-le-champ-tags-est-en-ecriture-seule/initiative.md) | `1-backlog` | carte isolée | 1 |
| [K167](6-prete/K167-fragments-de-rapport-relient-la-feuille-partagee/initiative.md) | `6-prete` | carte isolée | 1 |
| [K168](1-backlog/14-K168-bandeau-d-inscription-en-attente-inexistant/initiative.md) | `1-backlog` | carte isolée | 1 |
| [K169](6-prete/K169-reserver-la-place-sur-la-construction-d-equipe/initiative.md) | `6-prete` | carte isolée | 1 |
| [K170](1-backlog/15-K170-le-bundle-css-est-gele-au-demarrage/initiative.md) | `1-backlog` | carte isolée | 1 |
| [K171](1-backlog/16-K171-l-avatar-d-un-coach-n-existe-nulle-part/initiative.md) | `1-backlog` | carte isolée | 1 |
| [K172](1-backlog/17-K172-cinq-swap-qui-imbriquent-app-content/initiative.md) | `1-backlog` | carte isolée | 1 |
| [K173](1-backlog/18-K173-seed-accounts-efface-le-legacy-id/initiative.md) | `1-backlog` | carte isolée | 1 |
| [K174](1-backlog/19-K174-le-widget-spp-retombe-en-silence/initiative.md) | `1-backlog` | carte isolée | 1 |
| [K175](6-prete/K175-un-correctif-js-peut-ne-jamais-atteindre-l-utili/initiative.md) | `6-prete` | carte isolée | 1 |
| [K176](1-backlog/20-K176-le-site-en-francais-et-en-anglais/initiative.md) | `1-backlog` | carte isolée | 1 |
| [K177](1-backlog/21-K177-sentry-l-alerte-que-le-journal-ne-donne-pas/initiative.md) | `1-backlog` | carte isolée | 1 |
| [K178](1-backlog/22-K178-archiver-une-saison/initiative.md) | `1-backlog` | carte isolée | 1 |
| [K179](6-prete/K179-le-plafond-de-participants-n-a-jamais-servi/initiative.md) | `6-prete` | carte isolée | 1 |
| [K180](1-backlog/23-K180-rendre-accessible-un-espace-publiquement/initiative.md) | `1-backlog` | carte isolée | 1 |
| [K181](6-prete/K181-un-roster-introuvable-disparait-sans-un-mot/initiative.md) | `6-prete` | carte isolée | 1 |
| [K182](6-prete/K182-changer-le-logo-d-une-equipe/initiative.md) | `6-prete` | carte isolée | 1 |
| [K183](6-prete/K183-donner-un-portrait-a-un-joueur/initiative.md) | `6-prete` | carte isolée | 1 |
| [K184](7-livree/K184-le-rapport-de-match-nait-toujours-d-un-apparieme/initiative.md) | `7-livree` | carte isolée | 1 |
| [K185](7-livree/K185-un-sens-de-flux-par-cas-metier/initiative.md) | `7-livree` | carte isolée | 1 |
| [K186](1-backlog/24-K186-l-agregat-team-n-a-pas-de-frontiere/initiative.md) | `1-backlog` | carte isolée | 1 |
| [K187](1-backlog/25-K187-match-day-expose-ses-appariements/initiative.md) | `1-backlog` | carte isolée | 1 |
| [K188](1-backlog/26-K188-les-journaliers-sans-coups-de-pouce/initiative.md) | `1-backlog` | carte isolée | 1 |
| [K189](6-prete/K189-le-middleware-csrf-n-existe-pas/initiative.md) | `6-prete` | carte isolée | 1 |
| [K190](6-prete/K190-le-nom-du-coach-vient-du-navigateur/initiative.md) | `6-prete` | carte isolée | 1 |
| [K191](6-prete/K191-la-session-ne-glisse-pas/initiative.md) | `6-prete` | carte isolée | 1 |
| [K192](1-backlog/27-K192-le-recapitulatif-annonce-un-faux-blocage/initiative.md) | `1-backlog` | carte isolée | 1 |
| [K193](1-backlog/28-K193-des-hx-vals-lisent-l-evenement-au-mauvais-moment/initiative.md) | `1-backlog` | carte isolée | 1 |
| [K194](6-prete/K194-le-brouillon-d-equipe-emprunte-une-competition/initiative.md) | `6-prete` | carte isolée | 1 |
| [K195](8-abandonnee/K195-appstate-test-duplique/initiative.md) | `8-abandonnee` | carte annulée | 1 |
| [K196](8-abandonnee/K196-mock-data-competition-detail/initiative.md) | `8-abandonnee` | carte annulée | 1 |
| [K197](8-abandonnee/K197-costly-mistakes/initiative.md) | `8-abandonnee` | carte annulée | 1 |
| [K198](8-abandonnee/K198-my-teams-page/initiative.md) | `8-abandonnee` | carte annulée | 1 |
| [K199](8-abandonnee/K199-team-customisation-admin/initiative.md) | `8-abandonnee` | carte annulée | 1 |
| [K200](8-abandonnee/K200-player-value-changed-listener/initiative.md) | `8-abandonnee` | carte annulée | 1 |
| [K201](8-abandonnee/K201-team-treasury-tab/initiative.md) | `8-abandonnee` | carte annulée | 1 |
| [K202](8-abandonnee/K202-reference-BC-roster-editor/initiative.md) | `8-abandonnee` | carte annulée | 1 |
| [K203](8-abandonnee/K203-tc-finalization-page/initiative.md) | `8-abandonnee` | carte annulée | 1 |
| [K204](8-abandonnee/K204-cross-bc-reference-repository-violations/initiative.md) | `8-abandonnee` | carte annulée | 1 |
| [K205](8-abandonnee/K205-customisation-onglet-actif-perdu/initiative.md) | `8-abandonnee` | carte annulée | 1 |
| [K206](8-abandonnee/K206-spp-disponibles-non-mis-a-jour/initiative.md) | `8-abandonnee` | carte annulée | 1 |
| [K207](8-abandonnee/K207-gestion-de-la-haine/initiative.md) | `8-abandonnee` | carte annulée | 1 |
| [K208](8-abandonnee/K208-reprendre-un-rapport-de-match-en-cours/initiative.md) | `8-abandonnee` | carte annulée | 1 |
| [K209](8-abandonnee/K209-une-erreur-js-a-l-ouverture-du-recrutement/initiative.md) | `8-abandonnee` | carte annulée | 1 |
| [K210](8-abandonnee/K210-implémenter-les-notifications-email/initiative.md) | `8-abandonnee` | carte annulée | 1 |
| [K211](8-abandonnee/K211-team-matches-tab/initiative.md) | `8-abandonnee` | carte annulée | 1 |
| [K212](7-livree/K212-fiche-equipe-tresorerie-et-matchs/initiative.md) | `7-livree` | cartes livrées de E06 | 7 |
| [K213](7-livree/K213-noms-sortir-de-la-liste-blanche/initiative.md) | `7-livree` | carte livrée de E07 | 1 |
| [K214](7-livree/K214-chacun-son-equipe-jusqu-aux-gains/initiative.md) | `7-livree` | cartes livrées de E22 | 7 |
| [K215](8-abandonnee/K215-dette-technique-diffuse-cartes-retirees/initiative.md) | `8-abandonnee` | cartes retirées de K49 | 3 |

## Décisions de l'humain (2026-11-19, reprise après le premier rangement)

1. **Cartes livrées dans une initiative en cours** (E06, E07, E22) : séparées dans une initiative
   à part, en `7-livree/` (K212, K213, K214). L'initiative d'origine garde ses cartes à livrer.
2. **K14** : ses dix cartes sont livrées, et le critère « Terminé quand » est couvert par
   `tests/e2e/test_competition_settings_ranking.py` ; elle passe en `7-livree/`.
3. **Initiatives définies en `1-backlog/`** : tolérées pour le moment, au-delà de la limite de
   trois initiatives en cours ; on vise la convergence à court terme.
4. **`6-prete/` relu** : K49 redescend en `5-tickets/` (cartes aux chemins périmés, la 14
   semble faite) ; K184 (552) et K185 (555) sortent en `1-backlog/` car des commits `feat`
   portent leur numéro. Les autres cartes isolées restent prêtes.
5. **Séries de cartes livrées** : découpage gardé pour l'archive.
6. **`docs/specs/`** reste en place, comme description de l'existant, mise à jour à chaque
   livraison ; les initiatives y renvoient par leur clé `conception:`.
7. **Outils du projet qui citent `kanban/`** : mis à jour dans un commit à part.
8. **Documentation** : `7-livree/` est l'archive ; `docs/specs/` décrit l'existant.

## Décisions de l'humain (2026-11-19, seconde reprise)

1. **K184 (552) et K185 (555)** : livrées, confirmé par l'humain ; elles passent en `7-livree/`.
2. **K49** : la carte 14, déjà faite, est abandonnée ; les cartes 09 et 11 sont réécrites sur le
   code actuel sous de nouveaux identifiants, `670-entity-sans-eq-concurrente` et
   `671-initials-une-seule-definition`. Les trois anciennes cartes sont rangées dans K215, en
   `8-abandonnee/`. K49 revient en `6-prete/`.
3. **Outils qui citent `kanban/`** : commit séparé, une fois ce rangement accepté (remplacer
   `kanban/` par `specs/` dans les quatre fichiers, réécrire la section épics de `CLAUDE.md` à
   partir de `specs/README.md`).
4. **`docs/specs/`** : rien de global dans les fichiers de contexte des agents ; chaque
   initiative y renvoie par sa clé `conception:`. Un dossier n'est déclaré que si une initiative
   en a besoin.

## Décisions de l'humain (2026-11-19, troisième reprise)

1. **Portée de la carte 671** (la copie `initials_from` du domaine `teams`) : ce n'est pas une
   question de rangement. La carte reste telle qu'elle a été écrite ; sa portée se règle
   au moment de la réaliser, pas pendant l'onboarding.
2. **Cartes 09 et 11 remplacées** : gardées dans K215 (`8-abandonnee/`), ce qui conserve leur
   historique et réserve leurs numéros.
