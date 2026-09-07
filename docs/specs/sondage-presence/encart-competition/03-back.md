# Phase 3 — Architecture back : l'encart du coach connecté

**Entrée** : `02-front.md` validé — un widget, une action, aucun événement DOM,
une carte par campagne ouverte.

## Ce que l'existant impose, et qui a été vérifié

| Fait | Conséquence |
|---|---|
| `competition-detail.html` compose déjà six onglets par `hx-get` | l'encart s'ajoute en conteneur, la struct de page ne gagne aucun champ |
| `space_scope` a un résolveur pour `competition_id` | le chemin de l'encart le porte, l'espace est donc contrôlé |
| `require_admin_access` fait **deux** choses | l'encart n'a besoin que de la seconde — cf. ci-dessous |
| `find_enrolled_teams` et `find_team_names` existent | aucun port à créer |
| `IPresenceSurveyRepository` existe (unité 1) | il lui manque une seule requête |

## La garde du non-admin n'existe pas

`require_admin_access` vérifie le droit d'administration **et** que la saison
appartient à la compétition. L'encart a besoin de la seconde sans la première :
n'importe quel coach le voit, et un `season_id` d'une autre compétition doit
rendre `404`.

Or cette moitié est aujourd'hui `verifier_saison_de_la_competition`, **privée**
dans `admin_page.rs`.

**Forme retenue** : la déplacer dans `admin_scope.rs`, où vivent déjà ses quatre
sœurs — `journee_de_la_saison`, `appariement_de_la_saison`,
`groupe_de_la_saison`, `equipe_de_la_saison`. Ce sont toutes des vérifications
« cette ressource appartient-elle à ce parent », et celle-ci en est une.

Le déplacement se fait par **copier-coller exact**, imports adaptés, jamais
réécrit — c'est la règle 5 du `CLAUDE.md`, et son `404` volontaire a un motif
écrit qu'une réécriture perdrait : *une saison qui n'appartient pas à cette
compétition est hors du périmètre du chemin ; répondre 403 confirmerait son
existence à qui essaie des identifiants.*

**Le nom du module devient discutable** — `admin_scope` importé par un handler
qui n'est pas d'administration. C'est un défaut de nom, pas de conception : les
quatre fonctions n'ont jamais rien vérifié d'administratif. Le renommer touche
une dizaine d'imports et n'appartient pas à ce chantier ; **cela mérite sa
carte**, hors épic.

## Ce que l'encart ne divulgue pas

Il ne montre que **les équipes du demandeur**. Un coach qui n'a aucune équipe
engagée dans la saison ne voit rien — pas un encart vide, rien.

C'est ce qui rend inutile une garde d'appartenance à l'espace : le widget ne
peut rien révéler qui ne soit déjà au demandeur. La question « qui a le droit de
voir la page de détail d'une compétition » reste entière, et elle est
antérieure à ce chantier.

## Plan de fichiers

| Fichier | Contenu |
|---|---|
| `io/web/widgets/presence_call_widget.rs` | le widget et son action |
| `io/web/templates/widgets/presence-call.html` | l'encart, ses deux mises en forme |
| `use_cases/presences/presence_call_service.rs` | l'hydratation |
| `domain/presence_survey_repository_port.rs` | **modifié** — une requête de plus |
| `io/repository/sql/presences/list_open_surveys_for_season.sql` | la requête |
| `io/web/templates/competition-detail.html` | **modifié** — le conteneur `hx-get` |
| `io/web/admin/admin_scope.rs` | **modifié** — accueille `saison_de_la_competition` |
| `io/web/admin/admin_page.rs` | **modifié** — la perd, et l'importe |
| `assets/static/css/pages/presence-call.css` | + inscription au bundle |
| `routes.rs` | deux constantes |

**Le widget et son action dans un seul fichier.** L'onglet d'administration en
séparait trois — onglet, widgets, actions — parce qu'il portait deux widgets et
neuf actions. Ici il y a un widget et une action qui rend le même fragment :
les séparer ferait deux fichiers dont l'un compterait vingt lignes.

## La requête qui manque

```rust
async fn list_open_surveys_for_season(&self, season_id: &str, maintenant: &str)
    -> Result<Vec<PresenceSurvey>, PresenceSurveyRepositoryError>;
```

Une requête pour la saison, pas une par journée — même motif que
`list_summaries` de l'unité 1 : l'encart doit connaître toutes les campagnes
ouvertes, et les chercher journée par journée ferait vingt allers-retours.

**Elle rend des agrégats, pas un DTO de lecture**, parce que l'encart a besoin
de `statut()`, de la réponse de chaque équipe et de son horodatage — trois
questions du domaine. Un DTO les aurait aplaties, et le VM aurait dû les
reconstituer.

**Le filtre « ouverte » est dans la requête, et c'est un compromis assumé.** R23
rend la clôture calculable depuis l'échéance et `close_le`, donc le SQL peut
écarter d'emblée les campagnes échues ou closes — et `statut()` retranche ensuite
ce qui reste. La règle vit à deux endroits, mais l'alternative — charger toutes
les campagnes de la saison pour en jeter la plupart — ferait payer chaque
affichage de page pour rien. Le SQL **élague**, le domaine **décide** : c'est le
domaine qui reste la référence, et un désaccord ne produit qu'une campagne
chargée pour rien, jamais une campagne affichée à tort.

## Le service d'hydratation

```rust
// arch:no-instrument — service d'hydratation : assemble une vue, sans intention métier
pub async fn hydrater(
    season_id: &SeasonId,
    coach_id: &CoachId,
    survey_repo: &dyn IPresenceSurveyRepository,
    team_port: &dyn ITeamInfoPort,
    maintenant: …,
) -> Vec<CampagneOuverte>
```

Il croise les campagnes ouvertes, les équipes engagées du coach
(`find_enrolled_teams` filtré sur `coach_id`) et les réponses, et rend des objets
locaux — `TeamInfoDto` ne sort pas d'ici.

C'est lui qui applique la règle de composition de la phase 2 : **une campagne
n'est retenue que si le coach y a au moins une équipe**, et les campagnes sont
ordonnées par échéance croissante.

## Les routes

```
/app/{space_id}/competitions/{competition_id}/{season_id}/presence-call
                                                          …/presence-call/answer
```

Dans le routeur **protégé** — c'est le seul chemin des trois unités qui exige une
session. L'action porte `{ round_id, team_id, presence }` dans son corps, comme
l'action `answer` de l'onglet d'administration, et appelle le même
`record_answer_use_case` avec `Repondant::Coach(id du connecté)` (R28).

## L'encart disparaît à la clôture, et c'est voulu

Une campagne close ne produit aucune carte : le coach n'a plus rien à y faire.

**Ce n'est pas un trou à combler par un sixième état.** Le coach qui veut vérifier
ce qui est enregistré à son nom a son lien d'e-mail, qui affiche justement sa
dernière réponse avec le motif de la fermeture (R27). Ajouter ici un récapitulatif
en lecture seule ferait un encart permanent sur une page où l'on vient lire un
classement.

## Règle métier apparue en phase 3

Aucune. La phase 2 a produit R28, qui était la seule question ouverte ; celle-ci
n'a rencontré que des choix de placement.

C'est le signe attendu d'une troisième unité : l'agrégat, les use cases et les
ports existent, et il ne reste qu'à les brancher sur un écran de plus.
