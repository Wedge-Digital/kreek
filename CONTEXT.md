# Domain Context — Kreek

Glossaire de la langue du domaine. Un terme par ligne, une définition métier — ni structure de code, ni décision technique.

## Espace & Coach

| Term | Definition |
|---|---|
| **Space** | Espace de jeu privé regroupant des coaches, leurs équipes et des compétitions. Toutes les données de jeu appartiennent à un espace ; rien n'est partagé entre deux espaces. |
| **User** | Compte d'authentification. Un même user peut être coach dans plusieurs espaces. |
| **Coach** | Rôle d'un user dans un espace : il y gère une ou plusieurs équipes. |
| **Access Mode** | Manière dont une compétition recrute ses participants : sur **invitation** (seuls les coaches invités peuvent s'inscrire) ou **ouverte** (tout coach de l'espace peut demander à s'inscrire). |
| **Invitation** | Proposition faite à un coach de rejoindre une compétition. Une inscription peut exiger une validation de l'organisateur, et la compétition peut plafonner le nombre de participants ou fixer une date limite d'inscription. |

## Competition

| Term | Definition |
|---|---|
| **Competition** | Ligue ou tournoi organisé dans un espace. Contient des saisons. |
| **Season** | Période de jeu d'une compétition. Contient des journées et porte le classement. |
| **Round (Match Day)** | Journée de match d'une saison — un seul concept, désigné « journée » côté calendrier et « round » quand un match ou une ligne de classement s'y rattache. Contient des pairings, sauf si c'est une **journée de repos** (aucun match programmé). Une journée est datée soit à date fixe, soit sur une plage de dates dans laquelle les coaches conviennent de leur match. |
| **Pairing** | Affrontement programmé entre deux équipes dans une journée. Génère automatiquement un Match Report en état Draft. |
| **Ranking Group** | Poule au sein d'une saison : les équipes y sont classées entre elles. La répartition des équipes dans les poules est automatique ou décidée par l'organisateur. |
| **Playoffs Phase** | Phase finale à élimination jouée après les journées de saison, alimentée par un nombre défini de qualifiés par poule. Peut inclure un match pour la troisième place. |
| **Tier** | Palier d'entrée d'une compétition : il fixe le budget de création d'équipe, l'expérience de départ des joueurs, et restreint les rosters, inducements et Star Players autorisés. |
| **Competition Rules** | Ensemble des règles de la compétition choisies par l'organisateur : barème de points de classement, bonus, tiers. |

## Team

| Term | Definition |
|---|---|
| **Team** | Équipe créée par un coach, inscrite dans une saison de compétition. |
| **Roster** | Type d'équipe (Orcs, Elfes Sylvains, etc.). Définit les positions de joueurs disponibles et les règles spéciales applicables. |
| **Participation Status** | Situation d'une équipe vis-à-vis d'une compétition : inscription **en attente**, **inscrite**, **retirée** ou **refusée**. |
| **Game Phase** | Étape du cycle de vie d'une équipe entre deux matchs : prête à jouer (ReadyToPlay), saisie du rapport de match (MatchReporting), amélioration des joueurs (PlayerImprovement), recrutement (Recruitment), licenciements (Dismissals), retraite temporaire (TemporaryRetirement), inter-saison (OffSeason). |
| **Treasury** | Trésorerie de l'équipe, en kPo. Alimentée par les gains de match, dépensée en recrutement, staff et inducements. |
| **kPo** | Unité monétaire du jeu (milliers de pièces d'or). Toutes les valeurs et coûts sont exprimés en kPo. |
| **Team Value (TV)** | Valeur totale de l'équipe = somme des valeurs de ses joueurs et de son staff. |
| **Current Team Value (CTV)** | Team Value au moment d'un match donné, utilisée pour déterminer topdog et underdog et calculer le budget d'inducements. |
| **Dedicated Fans** | Nombre de fans permanents d'une équipe (au plus 20). Évolue match après match selon le modificateur de fan factor. |
| **Staff** | Éléments d'encadrement achetés par l'équipe : relances (rerolls), apothicaire, assistants, pom-pom girls, et achat de fans dévoués. |
| **Costly Mistake (Incident)** | Incident de gestion qui ponctionne une trésorerie trop importante en inter-saison, d'intensité croissante : aucun, mineur, majeur, catastrophe. |

## Player

| Term | Definition |
|---|---|
| **Player** | Joueur recruté par une équipe sur une position de son roster, identifié par son numéro de maillot. Sa carrière (progression, blessures, statistiques) le suit tant qu'il appartient à l'équipe. |
| **Position** | Poste du roster occupé par un joueur (Lineman, Blitzer, etc.). Fixe ses caractéristiques de départ, son coût et ses catégories de compétences accessibles. |
| **SPP (Star Player Points)** | Points d'expérience gagnés par un joueur pour ses actions en match. Ils constituent le pool qu'il dépense pour progresser. |
| **Player Value** | Valeur du joueur en kPo : son coût de position augmenté des améliorations acquises. Contribue à la Team Value. |
| **Improvement** | Progression achetée avec les SPP d'un joueur : une nouvelle **compétence**, ou une **augmentation de caractéristique** (MA, ST, AG, PA, AV). Chaque amélioration a un coût en SPP et augmente la valeur du joueur. |
| **Acquisition Mode** | Manière dont une compétence est obtenue : **choisie** par le coach ou tirée **aléatoirement**. Le mode change le coût en SPP. |
| **Injury** | Conséquence d'une agression subie en match, de la plus légère à la plus définitive : commotion, amoché, blessure sérieuse, séquelle, mort. |
| **Sequel (Séquelle)** | Blessure permanente qui inflige un malus définitif sur une caractéristique du joueur (MA, ST, AG, PA, AV). |
| **Player Participation Status** | Disponibilité d'un joueur : **disponible**, **absent au prochain match**, **retiré** ou **mort**. |
| **Career Counters** | Statistiques cumulées d'un joueur sur toute sa carrière : matchs joués, touchdowns, passes, interceptions, sorties infligées, MVP, fautes, blessures persistantes. |

## Match Report

| Term | Definition |
|---|---|
| **Match Report** | Agrégat qui suit le cycle de vie complet d'un match : sélection des équipes, séquence d'avant-match, déroulement, résultat, publication. |
| **Origin** | Provenance d'un match report : créé **manuellement** par un coach, ou automatiquement à partir d'un **pairing** de journée. |
| **Draft** | État initial du match report — les équipes sont assignées mais la sélection n'est pas confirmée. |
| **PreMatch** | État après confirmation — les équipes sont verrouillées, la séquence d'avant-match puis la saisie des actions peuvent commencer. |
| **ReadyToPublish** | État atteint quand l'après-match est saisi : gains, modificateurs de fans et résumé sont connus, mais rien n'est encore diffusé. Reste corrigible. |
| **Published** | État final — le rapport est diffusé. Il devient la source des conséquences pour les autres domaines : classement, progression des joueurs, trésorerie et fans des équipes. |
| **Cancelled** | État terminal d'un match report abandonné avant publication, avec un motif d'annulation. |
| **Pre-Match Sequence** | Enchaînement obligatoire avant le premier tour : facteur fans, valeurs d'équipe, inducements, mise en place des joueurs temporaires. |
| **Fan Factor** | Nombre total de fans d'une équipe pour un match = ses Dedicated Fans + un jet de D3 saisi par le coach. |
| **Fan Factor Mod** | Variation des Dedicated Fans d'une équipe à l'issue du match, entre -2 et +2. |
| **Topdog** | Équipe dont la CTV est la plus haute. Elle achète ses inducements en premier, avec sa seule trésorerie. |
| **Underdog** | Équipe dont la CTV est la plus basse. Elle achète en second, avec un budget élargi : écart de CTV + dépenses du topdog + sa propre trésorerie. |
| **Inducements** | Achats d'avant-match (pots-de-vin, Star Players, mercenaires, etc.) valables pour ce seul match, limités par le budget d'inducements de l'équipe et par la quantité maximale autorisée pour chaque inducement. |
| **Temporary Player** | Joueur présent pour ce seul match, jamais recruté par l'équipe. Trois natures : **Star Player** (engagé via un inducement), **Mercenary** (mercenaire engagé via un inducement), **Journeyman** (recruté d'office pour compléter un effectif de moins de 11 joueurs disponibles, sur la position de journeyman du roster). Ses actions comptent dans le match mais ne construisent pas de carrière. |
| **Turn** | Tour de jeu auquel une action est rattachée, de 1 à 16 (huit tours par équipe et par mi-temps). |
| **Match Action** | Fait de jeu enregistré dans le rapport, attribué à un joueur d'un des deux camps et à un tour : touchdown, passe, interception, agression, lancer de coéquipier, sortie, MVP, blessure infligée. |
| **Casualty (Sortie)** | Action qui met un joueur adverse hors du terrain. Le nombre de sorties infligées par une équipe conditionne le bonus agressif au classement. |
| **MVP** | Joueur désigné meilleur joueur du match. Rapporte des SPP à son bénéficiaire. |
| **Score** | Nombre de touchdowns d'une équipe sur le match, déduit de ses actions. Détermine le résultat (victoire, nul, défaite). |
| **Match Gain** | Recette du match versée à la trésorerie d'une équipe. |
| **Match Summary** | Titre et récit libres du match, saisis à l'après-match pour accompagner la publication. |

## Ranking

| Term | Definition |
|---|---|
| **Ranking** | Classement des équipes d'une saison, ordonné par points de classement. Il n'existe qu'à partir de matchs publiés. |
| **Ranking Line** | Fait immuable enregistré pour une équipe à l'issue d'un match publié : ses compteurs cumulés depuis le début de la saison (matchs joués, victoires, nuls, défaites, points). La dernière ligne d'une équipe fait foi pour le classement affiché. |
| **Ranking Points** | Points de classement d'une équipe : cumul, match après match, des points de résultat et des points bonus. |
| **Points Scale** | Barème de la compétition : points attribués pour une victoire, un nul et une défaite. |
| **Offensive Bonus** | Point(s) supplémentaire(s) accordé(s) à une équipe ayant marqué au moins un nombre donné de touchdowns. Activable par compétition. |
| **Defensive Bonus** | Point(s) supplémentaire(s) accordé(s) à une équipe n'ayant pas encaissé plus qu'un nombre donné de touchdowns. Activable par compétition. |
| **Aggressive Bonus** | Point(s) supplémentaire(s) accordé(s) à une équipe ayant infligé strictement plus qu'un nombre donné de sorties. Activable par compétition. |

## Reference Data

| Term | Definition |
|---|---|
| **Reference Data** | Données de règles du jeu, communes à tous les espaces et non modifiables par les coaches : rosters, positions, compétences, règles spéciales, Star Players, inducements, staff, ligues. |
| **Skill** | Compétence acquérable par un joueur, rattachée à une catégorie. Son coût en SPP dépend de la catégorie et de son accessibilité pour la position du joueur. |
| **Skill Category** | Famille de compétences (Générale, Agilité, Force, Passe, Mutation…). Une position y a accès normalement ou seulement au tirage aléatoire. |
| **Special Rule** | Règle particulière attachée à un roster ou à une ligue, qui modifie ce que l'équipe peut faire ou acheter. |
| **Star Player** | Joueur vedette engageable pour un seul match via un inducement, disponible pour certains rosters ou ligues seulement. |
| **League** | Univers de jeu de référence dont dépendent les rosters et Star Players disponibles. Choisie à la création d'une équipe. |

## News

| Term | Definition |
|---|---|
| **Article** | Publication rédigée par un user dans un espace : titre, chapô, image, et contenu en paragraphes typés. |
| **Article Tag** | Nature d'un article : actualité, compte-rendu de match, analyse, interview, tutoriel. |
| **Comment** | Réaction d'un user à un article. |
