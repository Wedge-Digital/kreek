# E19 — Saisie du compte rendu : droits et robustesse — cadrage

Fiche de cadrage de l'épic `kanban/epics/to_be_refined/E19-saisie-du-compte-rendu-droits-et-robustesse.md`,
validée le 2026-11-01. Une épic qui fait évoluer l'existant ne suit pas le
workflow feature page par page : elle part de ce que le code fait, des règles
qu'elle pose, des événements qu'elle touche, des routes et des tests.

## 1. Ce que le code fait aujourd'hui

- **Le service d'accès existe** : `src/app/match_report/use_cases/match_report_access_service.rs`.
  - `is_authorized` — admin d'espace ou de compétition (`is_admin`, le service
    commun), **ou** coach de l'une des deux équipes ;
  - `est_administrateur` — les admins seuls ;
  - le port `ITeamDataPort::is_coach_of_team` existe : **aucun port nouveau** ;
  - un port en erreur vaut refus : le contrôle échoue fermé.
- **Seuls le récapitulatif** (et sa publication, sa dépublication) **et l'écran
  de sélection** s'en servent. Toutes les autres routes, lecture comme
  écriture, ne vérifient que la session ; `get_step3`/`get_step4` n'extraient
  même pas `AuthSession`.
- **Le conflit se perd** : `ConcurrentWrite` n'existe qu'au dépôt ; les use
  cases le convertissent en `…Error::Repository(String)`, son type disparaît,
  et les contrôleurs rendent 500.
- **`published_by` existe** dans l'état `Published` (événement
  `MatchReportPublished`) ; le récapitulatif lit pourtant `created_by`, qui vaut
  l'identifiant de compétition pour tout brouillon issu du calendrier — et
  `build_submitted_by`, ne trouvant pas de coach, masque la ligne.

## 2. Les règles

| # | Règle |
|---|---|
| R1 | Toute **écriture** d'un compte rendu — sélection, avant-match, coups de pouce, actions, étape 5 — est réservée aux admins et aux coachs des deux équipes ; les autres reçoivent 403. |
| R2 | Les **écrans de saisie** et leurs widgets suivent la même règle : un écran qu'on ne peut pas soumettre ne se montre pas. |
| R3 | Le domaine sait dire le **camp** d'un coach : domicile, visiteur, les deux, aucun. E19 ne s'en sert que pour R1 ; E22 en fera la garde par camp. |
| R4 | Deux écritures simultanées donnent **409** et un message — jamais 500. |
| R5 | Le récapitulatif nomme **celui qui a publié**. |

**Ce qui ne change pas** (décision du 2026-11-01) : la création d'un match hors
calendrier (`/new`) et les feuilles imprimables gardent leur visibilité
actuelle.

## 3. Les événements

Aucun créé, aucun modifié. La carte 352 fait produire `MatchReportConfirmed`
par le publisher à partir du domain event `SelectionConfirmed`, qui existe ;
le contenu de l'app event reste identique.

## 4. Les routes

| Route | Aujourd'hui | Après E19 |
|---|---|---|
| `GET/POST /new` — hors calendrier | session | **inchangé** |
| `GET/POST /{id}` — sélection | `est_administrateur` pour figer | R1, plus la règle actuelle du hors-calendrier |
| `GET/POST /step2`, `/inducements/{team}` | session | R1, R2 |
| `GET /step3`, `/step4` et leurs widgets — tour, joueurs temporaires, panneau d'action, journal | rien | R2 |
| `POST /actions`, `DELETE /actions/{id}` | session | R1 |
| `GET/POST /step5` | session | R1, R2 |
| `/recap`, `/recap/publish`, `/recap/unpublish` | `is_authorized` | inchangé |
| Feuilles imprimables | session | **inchangé** |
| `GET /pairing/{id}` | redirection | inchangé — la page d'arrivée porte la garde |

## 5. Les données de production

| Carte | Données modifiées | Effet sur l'existant |
|---|---|---|
| 621 | aucune | lecture seule |
| 622 | aucune | un tiers qui saisit pour d'autres sans être admin est bloqué, ainsi que l'ancien coach d'une équipe reprise — accepté sans mesure préalable |
| 623 | aucune | le conflit est déjà refusé ; seule la réponse change |
| 624 | aucune | `created_by` n'est pas touché ; tous les récapitulatifs publiés, passés compris, nomment leur publieur |
| 352 | aucune en base | `teams` et `competitions` réagissent à `MatchReportConfirmed` : leurs réactions doivent supporter d'être rejouées |

## 6. Les tests

- **Unitaires** : le service d'accès sur une doublure du port — admin, coach
  domicile, coach visiteur, tiers, port en erreur ; la conversion du conflit.
- **E2E** : un tiers refusé sur une écriture et sur un écran de saisie, par
  l'en-tête `X-Bypass-Auth-Profile: simple` déjà employé ailleurs ; le
  récapitulatif nomme le publieur.
- **Non-régression** : la suite passe par DevCoach, admin ; `match_report_helpers.py`
  ne bouge pas.
