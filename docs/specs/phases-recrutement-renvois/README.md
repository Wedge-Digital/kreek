# Phases de recrutement et de renvois — Spec index

Construction des deux phases orphelines de la séquence d'après-match : `Recruitment`
(achat de joueurs, de staff et de relances) et `Dismissals` (renvoi de joueurs et de
staff). Deux pages dédiées, atteintes depuis les bannières de phase de la fiche
d'équipe.

Ces deux phases existent aujourd'hui dans la machine à états (`GamePhase`) et
affichent chacune une bannière qui **promet une fonctionnalité absente** :
« Achetez des joueurs ou du staff » avec pour seule action « Terminer les achats ».
La spec `team-state-management` les avait explicitement mises hors de son périmètre
(« Bouton *Recruter* omis », « Bouton *Gérer les renvois* omis ») ; cette feature en
est la suite directe.

## Hors périmètre explicite

- **Déplacement des journaliers** vers `teams` — lot séparé, à prendre après.
- **Calcul de la valeur d'équipe** — cartes 249 à 253, indépendantes.
- **Retraite temporaire** (`TemporaryRetirement`) — carte 39, non implémentée ;
  `DismissalsPhaseValidated` continue de transitionner directement vers `ReadyToPlay`.
- **Engagement définitif d'un journalier** — dépend du lot journaliers.

## Pages

| Page | Front | Back | DTOs | Use cases | Domaine | Intégration | Cartes |
|---|---|---|---|---|---|---|---|
| recrutement | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ |
| renvois | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ |

**17 cartes créées, 255 à 271** — voir `recrutement/08-cards.md` pour le tableau
complet et le chemin critique.

Les **48 règles métier** ont été récapitulées et validées à l'entrée de la phase 6.
Elles sont consolidées dans la section « Règles métier transverses » ci-dessous, et
chaque garde du domaine y renvoie par son numéro.

Maquettes validées (phase 1) : `assets/rawpages/html/app-team-recruitment.html` et
`assets/rawpages/html/app-team-dismissals.html`.

## Décisions d'architecture

| # | Décision |
|---|---|
| D1 | **Panier côté serveur**, pas côté client — conformité avec la construction d'équipe, et surtout règles métier écrites une seule fois, dans le domaine |
| D2 | Un **brouillon persisté** par phase et par équipe, sur le modèle de `team_roster_selections` |
| D3 | Le brouillon **ne touche jamais la trésorerie** : il accumule, la validation de phase émet le lot |
| D4 | **Version optimiste** sur le brouillon — première remontée d'un `ConcurrentWrite` jusqu'à l'utilisateur dans ce projet |
| D5 | À la validation, si le brouillon ne passe plus : **refus en bloc**, rien n'est appliqué |
| D6 | Les brouillons orphelins sont **purgés à chaque entrée en `ReadyToPlay`** |
| D7 | **Trésorerie en mouvements** : événements de crédit/débit nommés, pour disposer d'un historique consultable |
| D8 | **L'appartenance à l'effectif est un axe distinct** de la participation au match, dans le BC `players` — et la lecture par défaut exclut les renvoyés |

### D8 — l'appartenance à l'effectif

Renvoyer un joueur doit le faire cesser d'être compté, sans quoi la valeur d'équipe et
le nombre de journaliers restent faux. Or `players` n'a aucun moyen de l'exprimer.

`PlayerParticipationStatus` ne convient pas : il décrit des **conséquences de match**
— `player.rs:70` porte littéralement le commentaire « Impact des rapports de match ».
Un renvoi est une décision de coach. D'où un **axe distinct**, `RosterMembership`.

Vocabulaire unifié : la phase s'appelle `Dismissals`, l'événement devient
`PlayerDismissed` partout — y compris dans `teams`, où `PlayerFired` est renommé
(il n'est jamais émis, le renommage est gratuit).

**Le point qui protège dans la durée** : les sept chemins de lecture de l'effectif
excluent tous les renvoyés — le coach n'a pas besoin de les voir. `find_by_team_id`
filtre donc **à la source**, et aucun appelant n'a de filtre à écrire, donc aucun ne
peut l'oublier.

Détail complet dans `renvois/04-dtos.md`.

### D1 — pourquoi le panier serveur

Le panier client obligeait à écrire chaque règle **deux fois** : en JavaScript pour
désactiver les boutons, en Rust pour faire autorité. Cinq règles (max 16, quotas,
limites croisées, trésorerie, plancher de 11 éligibles), dix implémentations, deux
occasions de diverger. Le CLAUDE.md veut toute logique « est-ce autorisé ? » dans le
domaine.

Le panier serveur résout aussi la question de la session unique : le coach peut
acheter, quitter la page, revenir et acheter encore — ce que le panier client
interdisait, puisqu'il disparaissait à la moindre navigation.

## Règles métier transverses

### Trésorerie

- C'est la **seule** source de financement : on ne peut pas dépenser plus que ce dont
  on dispose. Vérification sur le **total du lot**, pas ligne par ligne.
- Elle est déjà dérivée des événements — `apply()` la mute en sept endroits — mais
  **aucun historique n'est consultable** : pas d'événement de mouvement nommé, pas de
  motif, pas de projection. C'est ce qui manque (D7).
- Un renvoi ne rembourse **rien**, ni joueur ni staff.

### Composition de l'effectif

- **16 joueurs maximum.**
- **Quota par poste** (`max_quantity` de la ligne de roster).
- **Limites croisées** — parfois « pas plus de N joueurs cumulés parmi ces postes ».
- Le poste doit appartenir au roster de l'équipe.
- **Plancher de 11 joueurs éligibles au prochain match** : on ne peut pas renvoyer en
  dessous. Un joueur absent ne comptant pas parmi les éligibles, il reste toujours
  renvoyable.
- À l'embauche, le joueur reçoit **le premier numéro de maillot disponible** —
  aucune saisie demandée au coach.

### Staff

| Élément | Achat | Renvoi |
|---|---|---|
| Relance | ✅ **au double** du prix de base hors création | ✅ |
| Assistant entraîneur | ✅ | ✅ |
| Pom-pom girl | ✅ | ✅ |
| Apothicaire | ✅ **si le roster y a droit** | ✅ même condition |
| Facteur fans | ❌ jamais | ❌ jamais |

- Quotas de `staff_fr.json` : apothicaire 1, assistants 6, pom-pom girls 6.
  **Relances : 8 maximum.**
- `allowed_staff` du roster conditionne l'apothicaire — **4 rosters sur 30** n'y ont
  pas droit : Rois des Tombes, Horreur Nécromantique, Nurgle, Morts-Vivants Titubants.

### Joueurs renvoyés

- Aucun remboursement.
- Le joueur perd ses **SPP et ses compétences acquises**.
- Un joueur absent au prochain match peut être renvoyé.

### Séquence

`PlayerImprovement → Recruitment → Dismissals → ReadyToPlay`

L'ordre est **voulu** : une équipe à 16 joueurs ne peut pas libérer une place et
recruter dans la même séquence. Elle renvoie cette fois-ci, recrute la suivante.

## Écarts constatés dans l'existant

Relevés pendant les phases 1 et 2, à traiter par cette feature :

| Écart | Emplacement |
|---|---|
| **Les limites croisées ne s'appliquent nulle part** — `cross_limits: vec![]` codé en dur, y compris à la création d'équipe | `team_creation/use_cases/roster_service.rs:68` |
| Les données de limites croisées ont **deux schémas incompatibles** : `{max, in}` pour 3 rosters, `{limit, limitedPlayerIds}` pour les Élus du Chaos — la struct Rust ne correspond qu'au second | `assets/references/teams_fr.json` |
| L'apothicaire est **renvoyable mais pas achetable** | `teams/domain/team.rs:663` (`StaffTypeNotBuyable`) |
| La relance est **achetable mais pas renvoyable** | `teams/domain/team.rs:684` (`StaffTypeNotDismissable`) |
| `allowed_staff` n'est **jamais consulté** par `teams` | — |
| Les **quotas de staff ne sont jamais vérifiés** | `buy_staff` ne teste que la trésorerie |
| Le **doublement du prix de relance n'existe pas** — le coût vient de la commande | `buy_staff` |
| `refund_kpo` **crédite la trésorerie** alors qu'un renvoi ne rembourse rien | `team.rs:477` |
| Recruter et licencier un joueur **n'existent pas** : `PlayerRecruited`, `PlayerFired` et `PlayerNotReEngaged` sont définis, appliqués, jamais construits | `teams/domain/team.rs` |

## Dépendances

### Avec la série 249-253 (valeur d'équipe)

| Lien | Nature |
|---|---|
| **Carte 251 → D6** | Elle crée le bus interne de `teams` et la publication depuis `TeamRepository::append`. Le listener de purge des brouillons s'y abonne : **sans elle, il n'y a rien à écouter**. |
| **D8 → cartes 250 et 253** | D8 **change la définition de « disponible »** : ce n'est plus seulement `participation_status`, c'est participation **et** appartenance. Livrées avant, ces cartes devront être retouchées ; livrées après, elles doivent intégrer les deux axes dès le départ. |
| **Carte 250 → ports** | Elle crée `IPlayerValuePort`. Cette feature l'élargit **deux fois** — ligne de roster au recrutement, identité et SPP aux renvois. À étendre, jamais à doubler. |

À ce stade ce n'est plus un port de valeur mais un **port de consultation de
l'effectif** : il devrait s'appeler `ISquadPort`, et le nom gagne à être posé dès la
carte 250 pour éviter un renommage ultérieur.

### Ordonnancement

La carte 251 est un **prérequis strict**. Les cartes 250 et 253 gagnent à être prises
**après** la décision D8, ou à l'intégrer d'emblée.

### Hors périmètre mais impacté

- **Carte 39** (retraite temporaire) : `Retired` reste sans producteur et son axe
  n'est **pas** tranché par D8. Une retraite temporaire n'est pas une fin
  d'appartenance.
- **Carte 46** (customisation admin) : ses futurs mouvements de trésorerie seront
  couverts d'office par D7, à condition que `treasury_movement()` reste un `match`
  exhaustif sans joker.
