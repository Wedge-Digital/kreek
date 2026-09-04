# Épics

Une **épic** est une grande fonction qui regroupe plusieurs cartes. Elle existe
pour une seule raison : garder une vue de haut niveau sur les chantiers, que la
liste plate des cartes ne donne pas.

L'épic dit **pourquoi** et **quand c'est fini**. Les cartes disent **comment**.

## Cycle de vie

```
to_be_refined → ready_to_be_done → en_cours → done
```

| Dossier | Contenu |
|---|---|
| `to_be_refined/` | Épic dont une carte au moins reste à raffiner |
| `ready_to_be_done/` | Toutes les cartes sont prêtes à implémenter |
| `en_cours/` | Au moins une carte est commencée, l'épic n'est pas close |
| `done/` | Le critère « Terminé quand » est vérifié |

L'état d'une épic suit **la plus en amont** de ses cartes : une épic dont neuf
cartes sont prêtes et une à raffiner reste en `to_be_refined/`.

## Conventions

- **Un fichier par épic**, nommé `E<NN>-<slug>.md`. Le préfixe `E` évite toute
  collision avec la numérotation des cartes.
- **Une carte appartient à une seule épic, ou à aucune.** Les cartes sans épic
  sont listées ci-dessous — on ne les range pas dans une épic « Divers », qui
  simulerait une vue de haut niveau au lieu d'en donner une.
- Chaque épic porte les sections : *La fonction* · *État* · *Les cartes* ·
  *Ce qui commande l'ordre* · *Ce que l'épic ne couvre pas* · *Terminé quand*.
- **« Terminé quand » est un critère observable**, jamais « toutes les cartes
  sont dans `done/` ». Une épic close se constate à l'écran ou à la mesure.

## Les épics

| Épic | État | Cartes |
|---|---|---|
| [E01 — Saison de jeu : le cycle de vie d'une équipe](done/E01-saison-de-jeu.md) | `done` | 10 |
| [E02 — Notifications e-mail de compétition](done/E02-notifications-email.md) | `done` | 13 |
| [E03 — Front : ni saut, ni clignotement](done/E03-front-ni-saut-ni-clignotement.md) | `done` | 4 |
| [E04 — Les verrous architecturaux](to_be_refined/E04-verrous-architecturaux.md) | `to_be_refined` | 6 |
| [E05 — Couverture e2e du déjà livré](to_be_refined/E05-couverture-e2e.md) | `to_be_refined` | 4 |
| [E06 — La fiche d'équipe complétée](to_be_refined/E06-fiche-equipe-completee.md) | `to_be_refined` | 8 |
| [E07 — Entrées utilisateur et identité](en_cours/E07-entrees-utilisateur-et-identite.md) | `en_cours` · 1/2 | 2 |
| [E08 — Mode customisation : finir la livraison](done/E08-mode-customisation.md) | `done` · fusionnée dans la 398 | 2 |
| [E09 — BC `news`](to_be_refined/E09-bc-news.md) | `to_be_refined` | 2 |
| [E10 — Référentiels éditables](to_be_refined/E10-referentiels-editables.md) | `to_be_refined` | 20 |
| [E11 — Savoir ce qui se passe en production](done/E11-journal-de-production.md) | `done` | 9 |
| [E12 — Administrer les membres d'un espace](done/E12-administrer-les-membres-d-un-espace.md) | `done` · 21/21 | 21 |
| [E13 — Gestion des erreurs coûteuses](done/E13-gestion-des-erreurs-couteuses.md) | `done` · 4/4 | 4 |
| [E14 — Modifier une compétition en cours](en_cours/E14-modifier-une-competition-en-cours.md) | `en_cours` · 7/10 | 10 |
| [E15 — Recruter un journalier](ready_to_be_done/E15-recruter-un-journalier.md) | `ready_to_be_done` · 0/6 | 6 |

## Les cartes sans épic

Elles ne relèvent d'aucune grande fonction. Les lister ici est le seul moyen
qu'elles ne disparaissent pas de la vue d'ensemble.

**Décisions de règle du jeu en attente** — ni l'une ni l'autre n'est un travail
de code tant que la règle n'est pas tranchée :

| Carte | Question ouverte |
|---|---|
| `274-inducements-egalite-de-tv` | À valeurs d'équipe égales, qui est top dog ? Un `>` strict désigne le domicile, personne ne l'a décidé |
| `473-une-option-anti-optimisateur` | Contenu à définir — trois portées possibles, sans rapport entre elles. Portait `TBD-62`, dont le numéro appartient à une carte close |

**Actions sur une équipe, hors cycle de saison** — sorties de E01 après
vérification : rien dans le code ne les implémente.

| Carte | État |
|---|---|
| `45-team-modification` | à raffiner |
| `46-team-customisation-admin` | à raffiner |

**Dette technique diffuse** — sept cartes courtes et indépendantes, chacune une
session brève : `06` (groupement O(n²)), `09` (`Entity::eq` shadowe
`PartialEq`), `10` (`bypass_auth` dans `AppState`), `11` (`initials()`
dupliquée), `14` (`cloudinary_transform()` privée), `15` (URLs par
`String::replace`), `254` (message d'erreur JSON trompeur).

**Isolées** :

| Carte | Note |
|---|---|
| `497-le-menu-du-selecteur-etait-rogne-par-son-panneau` | Le menu du `kreek-select` était coupé par l'`overflow: hidden` du panneau, posé pour ses coins arrondis — un `z-index` ne franchit pas un `overflow`. Et `getBoundingClientRect` ne voit pas ce défaut : le rectangle est le même, coupé ou non ; seul `elementFromPoint` dit ce qui est peint |
| `490-les-coachs-sont-deconnectes-trop-souvent` | Le cookie de session n'avait ni `Max-Age` ni `Expires` — le navigateur le jetait à sa fermeture, alors que le serveur gardait la session deux semaines. Et `SameSite: Strict` ne l'envoyait pas quand on arrivait par un lien externe. Deux lignes, plus la décision d'écarter Redis pour de bon |
| `491-les-sessions-ne-survivent-pas-a-un-redeploiement` | Le magasin est un `DashMap` que son propre en-tête réserve au dev : chaque redéploiement déconnecte tout le monde. En attente de mesure — on regarde d'abord ce que la 490 a réglé |
| `489-un-joueur-indisponible-se-voit-dans-la-liste` | Un joueur qui manquera le prochain match se lisait comme un joueur disponible : le statut vivait dans la projection depuis toujours et s'arrêtait au view model. Ligne barrée, pastilles pâlies, repère qui dit pourquoi — et la mesure mobile a montré que la cellule du nom débordait déjà de 88 px, sans rapport avec cette carte |
| `493-un-test-de-journalisation-accusait-le-produit` | La capture de journal ne recevait rien sur un runner à quatre cœurs, et le message accusait le use case ; l'instrument était muet. Cache d'intérêt de `tracing` reconstruit, et une ligne témoin fait désormais échouer l'instrument plutôt que le produit. Avec un `goto` e2e qui partait avant son POST |
| `498-la-competence-bonus-de-debut-de-ligue-ampute-le-solde-de-spp` ✅ | Une compétence prise sur les SPP bonus de début de ligue était débitée du solde du joueur, alors que ces points appartiennent à l'équipe — `spp_pool`, partagé — et que le joueur naît à zéro. Un joueur ayant pris Blocage à 6 puis gagné 8 SPP n'en affichait que 2, et ne pouvait pas dépenser ce qu'il avait réellement gagné |
| `495-l-ecran-annonce-moins-de-journaliers-qu-il-n-en-ajoutera` ✅ | Le bloc « Journaliers » de l'étape 2 comptait tout l'effectif là où la règle réelle ne retient que les alignables : l'écran annonçait moins de renforts que le rapport n'en ajouterait, pour un blessé comme pour un mort. Défaut préexistant, aggravé par la carte 488 qui a décidé que le mort appelle un journalier sans reprendre ce consommateur |
| `494-l-etape-2-affiche-deux-choses-fausses` ✅ | Deux défauts d'affichage sur l'étape 2 du rapport. `formatKpo` recomposait les montants depuis les milliers et les centaines, perdant le chiffre des dizaines : 2075 s'affichait « 2 000 kPo », et la différence de TV avec. Invisible en local, où aucune équipe n'atteint le seuil de 1 000. Et les deux jets de fan factor valaient 2 en dur, sous un bandeau « déjà enregistré » qui disait le contraire |
| `492-la-liste-affiche-les-spp-gagnes-au-lieu-du-solde` ✅ | La colonne SPP de la feuille d'équipe affichait le cumul des points gagnés : `players_proj.spp` ne fait que monter, `PlayerSkillPurchased` n'en retirant rien — seuls les chemins d'annulation le font. La fiche du joueur, elle, appelait déjà `spp_remaining()`. Deux écrans, deux chiffres pour la même chose |
| `488-un-mort-ne-compte-plus` ✅ | Un joueur tué en match reste affiché sur la fiche d'équipe et continue d'occuper sa place dans le plafond de seize et dans le quota de son poste : l'ACL de `teams` écrase les quatre statuts de participation en un booléen, où mort et blessé se confondent. Un blessé doit garder sa place — il revient au match suivant — donc le filtre ne peut pas se faire sur ce booléen |
| `487-du-css-qui-ne-trouve-pas-son-markup` | Six règles écrites pour un bouton posé sous le `</div>` de fermeture de son widget : elles ne rencontraient aucun markup, et le bouton était un lien nu dans les deux onglets de classement. Avec deux autres défauts du même écran et l'axe 17, qui refuse une classe stylée hors de sa racine — le contrôle au navigateur, lui, a été écarté sur mesure : 68 % des sélecteurs ne rencontrent rien sur les pages du harnais |
| `486-un-refus-de-depublication-ne-se-distingue-pas-d-un-succes` | Un refus de dépublication et un succès rendent le même `200`, le même `hx-refresh` et le même corps vide — et rien n'était journalisé. Le test postait sans regarder, attendait trente secondes et accusait le délai ; la CI de `demo` a été rouge trois runs sur quatre sur ce message-là |
| `485-modifier-une-competition-la-remet-en-construction` | Enregistrer un réglage depuis l'administration faisait redescendre la saison sous `ready` : la carte de la compétition renvoyait vers l'étape 2 du magicien et l'inscription se fermait, sans un mot — l'enregistrement, lui, réussissait. Le même piège avait déjà été corrigé deux fois, chaque fois pour le seul panneau où on l'avait vu ; l'axe 16 déduit désormais du SQL les méthodes qu'un panneau n'a pas le droit d'appeler |
| `484-cliquer-une-equipe-ne-fait-rien` | Cliquer une équipe depuis « Mes équipes » ne changeait rien à l'écran : la route rendait un fragment sous `HX-Request`, un en-tête vrai d'une navigation htmx comme d'un échange d'onglet. Cinq points d'entrée cassés, deux jours en production sous une suite verte |
| `483-le-montage-e2e-compte-les-clics-pas-les-embauches` | Trois copies d'une boucle de recrutement comptaient les clics derrière un délai fixe, pas les embauches enregistrées : l'assertion « 11 joueurs » passait sur dix, et `finalize_team` refusait vingt étapes plus loin. La CI de `demo` échouait une fois sur deux |
| `482-une-competence-gratuite-rencherit-la-suivante` | Une compétence donnée par un commissaire — ou une Haine — entrait dans la liste des compétences acquises, avec un coût nul, et faisait pourtant monter le niveau : la compétence suivante se facturait 8 SPP au lieu de 6, 16 après trois cadeaux. La même décision était déjà prise correctement pour les caractéristiques, dont les customisations vivent dans une liste à part |
| `480-l-axe-8-de-check-arch-ne-verifie-rien` | Un verrou bloquant affichait vert sans rien lire : il importait `tomllib`, absent avant Python 3.11, et le `python3` du système est en 3.9. Cinq tests e2e sans entrée dans la carte d'impact, que ni le local ni la CI — non déclenchée sur `demo` — ne voyaient |
| `18-script-inline-htmx-fragments` | Sortie de l'épic E03 à sa clôture : un `<script>` d'init dans un fragment ne peint rien sans ses styles et ne déplace rien, donc le critère de l'épic ne la mesurait pas |
| `474` et `475` — les statistiques de compétition | Le panneau des quatre tableaux Top/Flop sert de la donnée fictive **en production** — la seule dette visible par un utilisateur. La donnée existe dans `competition_match_display_proj`, table du BC lui-même : quatre tris d'une seule agrégation. Remplacent la carte 13, dont le constat sur l'onglet Équipes était périmé |
| `60-jersey-numbers-at-submission` | Attribution des numéros de maillot à la soumission |
| `362-le-bundle-css-est-gele-au-demarrage` | Une feuille éditée n'a aucun effet sur un serveur qui tourne, et rien ne le signale. A fait accuser à tort la carte 343 pendant une heure |
| `462-donner-un-portrait-a-un-joueur` | Voisine de la 461, mais elle traverse toute la pile : un joueur n'a aujourd'hui aucune image, et ce que la fiche affiche est son numéro de maillot sur un dégradé |
| `461-changer-le-logo-d-une-equipe` | Un logo choisi à la création ne se corrige nulle part. L'événement LogoChanged existe depuis l'origine dans l'agrégat, mais personne ne l'émet et rien ne le projette |
| `460-sous-total-des-joueurs-disponibles` | Une ligne de pied sous le tableau des joueurs. Elle porte aussi le marquage des indisponibles, sans lequel un total qui exclut une ligne sans dire laquelle paraîtrait faux |
| `454` à `459` — recruter un journalier | Un journalier devient un joueur dès le début du rapport de match : il agit, gagne des SPP, prend ses améliorations, et doit être recruté en phase de recrutement pour rester. Six cartes issues du workflow feature, spécifiées dans `docs/specs/embaucher-un-journalier/` |
| `449` à `453` — les points de classement manuels | Attribuer à une équipe des points qui ne viennent d'aucun match — forfait, sanction, rattrapage — et les rendre visibles et motivés. Cinq cartes issues du workflow feature, spécifiées dans `docs/specs/points-classement-manuels/`. La 451 demande la 448 |
| `448-deux-tokens-de-gris-indiscernables` | `--dark-6` et `--dark-7` diffèrent d'une unité par canal — rapport 1,0012. Le classement détaillé les oppose pour distinguer zébrage et survol : son survol est invisible une ligne sur deux |
| `438-un-roster-introuvable-disparait-sans-un-mot` | Un uid de roster que le corpus ne résout pas est écarté du sélecteur par un `filter_map` muet — trois causes distinctes produisent le même écran vide. Plus d'une heure de diagnostic en production |
| `427-un-rapport-manuel-en-cours-n-apparait-nulle-part` | La projection de l'onglet Résultats a `pairing_id` pour clef. Deux listeners fabriquent l'appariement manquant d'un rapport manuel, le troisième — celui qui écrit « en cours » — abandonne en silence |
| `415-le-plafond-de-participants-n-a-jamais-servi` | Un réglage que rien n'applique et que personne n'a jamais posé — 1874 saisons, zéro plafond. Il fait partir depuis toujours un e-mail portant une ligne « Places restantes » vide |
| `414-archiver-une-saison` | On n'implémentera pas de suppression : huit tables portent `season_id` et les trois flux d'événements n'en portent aucun. Une saison finie se range, elle ne se détruit pas — et l'archivage doit aussi taire le cron d'e-mails |
| `412-la-phase-finale-quitte-la-creation-de-competition` | Un réglage qu'il faut remplir et dont rien ne se sert : aucun appariement de phase finale n'est généré, aucun classement n'en tient compte |
| `397-sentry-l-alerte-que-le-journal-ne-donne-pas` | E11 a donné de quoi enquêter, pas d'être prévenu : une erreur en production n'existe que si quelqu'un ouvre les journaux et cherche |
| `395-le-site-en-francais-et-en-anglais` | ~1 700 chaînes : 144 templates, **869 libellés en Rust dont 120 dans le domaine**, 67 variantes d'erreur, 51 sélecteurs e2e. Instruite le 2026-08-26 — bilingue, catalogue plat, filtre Askama — puis mise en attente : trop volumineuse pour une session |
| `385-l-avatar-d-un-coach-n-existe-nulle-part` | Zéro utilisateur sur 864 en a un, aucun écran n'en pose, et le cache écrase la colonne. Trois tables entretiennent l'illusion |
| `361-reserver-la-place-sur-la-construction-d-equipe` | 1 265 px de saut en desktop, 1 841 en mobile — la plus grosse zone non réservée, hors périmètre de la 343 |
| `360-bandeau-d-inscription-en-attente-inexistant` | Un test e2e attend une classe qui n'a jamais été rendue. La suite est rouge en permanence |
| `357-le-champ-tags-est-en-ecriture-seule` | Quatre formes écrites, aucun lecteur : `find_by_tag()` n'a pas d'appelant. À trancher — compléter l'abstraction ou la supprimer |
| `352-match-report-confirmed-passe-par-le-publisher` | Deux use cases émettent un app event directement, ce que `CLAUDE.md` interdit. Trouvée par la carte 350. Débloque un axe « pas d'`app_event_bus` dans `use_cases/` » |
| `432-rendre-accessible-un-espace-publiquement` | Contenu à définir — « publiquement » recouvre trois portées, et la plus large touche `space_scope`, le verrou de la carte 416. Portait le numéro 61, déjà pris |
