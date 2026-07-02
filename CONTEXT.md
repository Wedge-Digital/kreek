# Domain Context — Kreek

## Match Report

| Term | Definition |
|---|---|
| **Match Report** | Agrégat qui suit le cycle de vie complet d'un match : sélection des équipes, séquence d'avant-match, déroulement, résultat. |
| **Draft** | État initial du match report — les équipes sont assignées mais la sélection n'est pas confirmée. |
| **PreMatch** | État après confirmation — les équipes sont verrouillées, la séquence d'avant-match peut commencer. |
| **Fan Factor** | Nombre total de fans pour un match = Dedicated Fans (donnée permanente de l'équipe) + jet de D3 (saisi par l'utilisateur). Calculé et persisté dans le match report pour chaque équipe. |
| **Dedicated Fans** | Nombre de fans permanents d'une équipe. Donnée gérée par le BC Teams. |
| **Journeymen** | Joueurs temporaires ajoutés automatiquement quand une équipe a moins de 11 joueurs disponibles. Le type de journeyman dépend du roster (Lineman par défaut). Ajoutés au pool de joueurs ayant participé au match à l'issue de la séquence d'avant-match. |
| **Current Team Value (CTV)** | Valeur de l'équipe au moment du match, utilisée pour calculer la différence de TV et le budget d'inducements. Donnée gérée par le BC Teams. |
| **Inducements** | Achats pré-match (Star Players, pots-de-vin, etc.). L'équipe avec la CTV la plus haute achète en premier, uniquement avec sa trésorerie (Petty Cash). L'équipe avec la CTV la plus basse achète en second, avec un budget = différence de CTV + dépenses adverses + sa propre trésorerie. |
| **Petty Cash** | Trésorerie de l'équipe disponible pour acheter des inducements. |

## Competition

| Term | Definition |
|---|---|
| **Competition** | Ligue ou tournoi organisé dans un espace. Contient des saisons. |
| **Season** | Période de jeu d'une compétition. Contient des journées (rounds). |
| **Round (Match Day)** | Journée de match dans une saison. Contient des pairings. |
| **Pairing** | Affrontement programmé entre deux équipes dans un round. Génère automatiquement un Match Report en état Draft. |

## Team

| Term | Definition |
|---|---|
| **Team** | Équipe créée par un coach, inscrite dans une saison de compétition. |
| **Coach** | Utilisateur qui gère une ou plusieurs équipes. |
| **Space** | Espace de jeu regroupant des coaches et des compétitions. |
| **Roster** | Type d'équipe (Orcs, Elfes Sylvains, etc.). Définit les positions de joueurs disponibles. |
| **Game Phase** | Phase de vie d'une équipe : recrutement, prête à jouer (ReadyToPlay), etc. |
