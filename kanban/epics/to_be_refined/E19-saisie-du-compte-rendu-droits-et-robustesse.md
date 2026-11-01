# E19 — Saisie du compte rendu : droits et robustesse

**État :** `to_be_refined` — 5 cartes, 4 faites (621 à 624) : 621 à 624, et 352 (à raffiner). Découpage
et cadrage validés le 2026-11-01.
**Cadrage :** `docs/specs/saisie-mobile/E19-cadrage.md`
Née le 2026-11-01 de l'analyse d'impact des maquettes mobiles
(`assets/rawpages/html/app-mobile-parcours.html`). Prérequis de E21 et E22.

## La fonction

Aujourd'hui, **tout membre connecté d'un espace peut saisir n'importe quel
compte rendu** de cet espace, pour n'importe quel camp. Seuls le récapitulatif,
la publication et la dépublication vérifient l'accès
(`match_report_access_service.rs`) ; l'avant-match, les coups de pouce,
l'ajout et la suppression d'actions, l'étape 5 et l'édition de la sélection
ne vérifient que l'existence d'une session.

Rien ne sait non plus dire si le coach connecté est celui du domicile ou du
visiteur : le camp d'une saisie vient toujours de l'URL (`step3`/`step4`,
`/inducements/{team_id}`).

Deux écritures simultanées sont bien détectées par l'event store
(`ConcurrentWrite`), mais finissent en 500 dans tous les contrôleurs.

L'épic ferme l'écriture aux deux coachs et aux admins, donne au domaine la
notion de camp du coach courant — dont E22 a besoin à chaque étape — et rend un
conflit lisible. Elle reprend au passage deux défauts du même BC trouvés par
l'analyse.

## Les cartes

| # | Intitulé | Apport |
|---|---|---|
| 621 | Le camp du coach courant | port vers `teams` qui répond domicile, visiteur ou aucun ; service d'accès par camp dans `match_report` |
| 622 | Les routes du compte rendu sont gardées | sélection, avant-match, coups de pouce, POST/DELETE d'actions, étape 5, et les écrans de saisie : coachs des deux équipes et admins (`is_admin`), 403 sinon |
| 623 | Un conflit d'écriture se dit | `ConcurrentWrite` → 409, message et rechargement, au lieu d'un 500 |
| 624 | « Publié par » au récapitulatif | se lit dans `published_by` de `MatchReportPublished`, et non plus dans `created_by` ; aucune donnée réécrite |
| 352 | `MatchReportConfirmed` passe par le publisher | existante ; deux use cases émettent l'app event en direct. E21 déplace la confirmation, autant la remettre d'aplomb avant |

## Ce qui commande l'ordre

Le camp du coach courant passe en premier : la garde des routes s'en sert, et
E22 la réutilise à chaque étape. Le conflit, « publié par » et la 352 sont
indépendants entre eux et de la garde.

## Conséquences sur les données de production

| Carte | Données modifiées | Effet sur l'existant |
|---|---|---|
| Camp du coach courant | aucune | aucun : lecture seule |
| Routes gardées | aucune | un tiers qui saisit aujourd'hui pour d'autres sans être admin sera bloqué, ainsi que l'ancien coach d'une équipe reprise. **Accepté sans mesure préalable** (décision du 2026-11-01) : les inconvénients ne sont pas majeurs |
| Conflit lisible | aucune | aucun : le conflit est déjà refusé, seule la réponse change |
| « Publié par » | aucune | **`created_by` n'est pas modifié** : il est écrit dans l'événement `MatchReportCreated` (immuable) et dans `match_report_proj.created_by`, et vaut l'id de compétition pour tout brouillon issu du calendrier — aucun coach n'est l'auteur d'un tirage. Le changer donnerait deux sens à la même donnée selon sa date. Aujourd'hui `build_submitted_by` ne trouve pas de coach pour ces brouillons et **masque** la ligne ; demain, tous les récapitulatifs publiés, passés compris, affichent « publié par » |
| 352 | aucune en base | `teams` (verrou de saisie) et `competitions` (statut, lien du compte rendu) réagissent à `MatchReportConfirmed` : si le publisher l'émet plus souvent qu'aujourd'hui, leurs réactions doivent supporter d'être rejouées. Contenu de l'événement inchangé |

## Ce que l'épic ne couvre pas

- **La lecture seule de l'autre coach en mode « Je saisis tout »** : elle
  dépend du mode de saisie, c'est E21.
- **La garde par camp du mode « Chacun son équipe »** : E22, qui s'appuie sur
  le camp du coach courant posé ici.
- **Le rafraîchissement de la vue de l'autre coach** : E23.
- **« Validé par »**, par camp : E22 — en « Je saisis tout », valider et
  publier ne font qu'un.
- **Les journaliers d'un match sans coups de pouce** : sortis de l'épic, carte
  620, sans épic.

## Terminé quand

Un membre de l'espace qui n'est ni l'un des deux coachs ni admin reçoit un 403
sur toute écriture d'un compte rendu, constaté par un test e2e. Deux écritures
simultanées sur le même compte rendu donnent un 409 et un message, jamais un
500. Le récapitulatif de tout compte rendu publié — y compris ceux publiés avant la
livraison — nomme le coach qui l'a publié.
