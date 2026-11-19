# Interdire les matchs hors calendrier, par compétition

**Priorité : haute — demandée après l'incident G. B. L. R**
**Épic :** aucune — une fonctionnalité, livrable d'un bloc
**Dépend de :** rien
**Maquette :** `assets/rawpages/html/app-competition-admin-settings-hors-calendrier.html`
**Fichiers :**
`src/app/competitions/domain/competition_options.rs` *(nouveau)*,
`src/app/competitions/domain/season_repository_port.rs`,
`src/app/competitions/io/repository/season_repository.rs`,
`src/app/competitions/io/web/admin/settings/general_options_panel.rs` *(nouveau)*,
`src/app/competitions/io/web/templates/admin/widgets/settings-general-options.html` *(nouveau)*,
`src/app/competitions/io/web/templates/admin/settings.html`,
`src/app/competitions/use_cases/settings/update_general_options_use_case.rs` *(nouveau)*,
`src/app/competitions/routes.rs`, `src/app/competitions/router.rs`,
`src/web/ports.rs` *(nouveau)*, `src/web/app_menu.rs`, `src/web/templates/app-menu.html`,
`src/infrastructure/web/hors_calendrier_adapter.rs` *(nouveau dossier)*,
`src/state.rs`, `src/main.rs`,
`src/app/match_report/io/web/match_selection_controller.rs`,
`src/app/match_report/io/web/templates/match-selection.html`,
`src/app/competitions/io/web/templates/competition-detail.html`,
`src/app/competitions/io/web/templates/admin/schedule.html`,
migration SQL (une colonne),
`tests/e2e/test_competition_hors_calendrier.py` *(nouveau)*, `tests/impact-map.toml`

## L'objectif

Une compétition peut **interdire les matchs hors calendrier**. Quand elle le
fait, plus personne n'y saisit de rencontre en choisissant lui-même les équipes :
seuls les commissaires créent des rencontres, par la génération d'appariements
du Calendrier.

## Ce qui l'a fait naître

L'incident G. B. L. R du 15 septembre. Un coach crée un rapport manuel sur la
journée 15, un appariement est fabriqué pour lui, quelqu'un le supprime, le
rapport survit — invisible au calendrier, vivant sur la fiche d'équipe. Le match
est ensuite reconstruit sur la journée 2.

Aujourd'hui **rien ne permet de fermer cette porte** : le hors-calendrier est
toujours possible, sans réglage. Cette carte donne le levier.

Elle **ne répare pas** le rapport orphelin : c'est la carte 552, indépendante.
Un hors-calendrier autorisé continuera de produire le même défaut.

## 1 · L'option et sa persistance

### Une colonne, pas un champ dans `structure`

`CompetitionStructure` porte `ranking_group` et `schedule` ; `CompetitionRules`
le classement et les tiers. Ni l'un ni l'autre n'accueille ce drapeau, et la
maquette lui donne sa propre section. **Une colonne JSONB de plus** sur
`competition_seasons`, aux côtés de `rules`, `structure`, `invitations` et
`notifications`.

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompetitionOptions {
    #[serde(default = "vrai")]
    pub autorise_hors_calendrier: bool,
}

fn vrai() -> bool { true }
```

**`#[serde(default)]` et non une valeur par défaut SQL.** La colonne sera
`NULL` sur toutes les saisons existantes, et c'est la désérialisation qui rend
`true`. Aucune reprise de données, et une saison écrite par une version
antérieure se relit sans erreur. Le motif est déjà en place sur
`aggressive_bonus` dans `RankingRules`.

**Le défaut est `true`** : au déploiement, aucune ligue ne change de
comportement. Interdire est un geste délibéré, jamais un effet de mise à jour.

### Le dépôt

Deux méthodes sur `ISeasonRepository`, sur le modèle exact de `find_invitations`
/ `save_visibility` : `find_options` et `save_options`. La seconde **ne touche
que sa colonne** — c'est ce que l'en-tête de `update_visibility_settings_use_case`
explique déjà à propos du statut de saison.

### La migration

Une colonne `options JSONB`, datée après la dernière du dossier.

**Le vrai piège n'est pas la date.** Sur une base importée de production, `sqlx`
refuse tout `migrate run` depuis `main` :

```
error: migration 20260908000001 was previously applied but is missing
       in the resolved migrations
```

`20260908000001` est le sondage de présence, appliqué en production, livré sur
`demo`, et absent du dossier de `main`. Rien à corriger : `cargo sqlx migrate
run --ignore-missing` passe, et le drapeau disparaîtra de lui-même à la
resynchro de `demo` sur `main`.

## 2 · Le panneau « Réglages généraux »

Sixième panneau de l'onglet ⚙️ Paramètres, **placé en deuxième position**, juste
après « Informations générales ».

Le modèle est `visibility_panel.rs`, et son en-tête dit pourquoi :

> « Le cinquième et dernier des cinq, et le plus sobre — **aucun JS**. Deux
> groupes de boutons radio dans un formulaire ordinaire […]. Ce qui peut être un
> `<form>` doit rester un `<form>`. »

Une case à cocher dans un `<form hx-post>` suit cette ligne exactement. À
recopier : le `Vm` avec son `from_domain()`, le `Template`, les deux handlers
`get_settings_general_options` / `post_settings_general_options` gardés par
`require_admin_access`, le `hx-disinherit="*"` sur la racine du panneau.

### L'encart de conséquence

Sous la case, visible en permanence :

> Décocher **n'efface aucun match déjà saisi** : les rencontres hors calendrier
> existantes restent visibles et modifiables. Seule la création de nouvelles est
> refusée.

Il répond à la question que la case pose sans y répondre. Sans lui, décocher
ressemble à une action destructive — et une case à cocher ne prévient pas.

## 3 · Les trois points d'entrée conditionnels

| | |
|---|---|
| `app-menu.html:33` | sous-menu desktop — « Saisir un Match » |
| `app-menu.html:109` | tabbar mobile — « Match » |
| `competition-detail.html:34` | bouton « + Saisir un match » |

**Desktop et mobile sont deux markups distincts**, pas un seul basculé par CSS :
deux conditions, pas une. C'est le genre d'erreur qui ne se voit qu'en réduisant
la fenêtre.

### La règle du menu, et son asymétrie assumée

**L'entrée disparaît pour tout l'espace dès qu'une seule compétition interdit le
hors-calendrier.**

Le menu est global à l'espace, l'option est par compétition. Un espace à trois
compétitions dont une seule interdit perd l'entrée partout, alors que la saisie
resterait légitime dans les deux autres. C'est voulu : un menu qui mènerait à un
formulaire refusé une fois sur trois serait pire.

`AppMenu` porte aujourd'hui un seul booléen conditionnel, `peut_administrer`,
dont le commentaire dit « la seule entrée de menu qui n'est pas offerte à
tous ». **Ce commentaire devient faux** — le corriger fait partie de la carte.

## 4 · Le port de la couche web

`src/web/` atteint `auth`, `spaces`, `news` (ses routes seulement) et
`shared_kernel`. Il n'a **jamais** touché `competitions`. Cette carte ouvre cette
dépendance, et elle passe par un port.

```
src/web/ports.rs                                   ← le trait, chez le consommateur
    #[async_trait]
    pub trait IHorsCalendrierPort: Send + Sync {
        /// Vrai si **au moins une** compétition de l'espace l'interdit.
        async fn un_espace_interdit(&self, space_id: &str) -> bool;
    }

src/infrastructure/web/hors_calendrier_adapter.rs  ← seul à importer competitions
```

**`src/infrastructure/web/` est un dossier nouveau, et le premier qui ne porte
pas un nom de BC.** Les huit autres — `competitions`, `match_report`, `news`,
`players`, `ranking`, `spaces`, `team_creation`, `teams` — sont des BCs
consommateurs. La règle dit « un sous-dossier par BC consommateur » ; ici le
consommateur est le layout web. `web/` a été préféré à `host/` parce qu'il nomme
le module qui consomme, et ce ne sera pas la dernière fois que le layout aura
besoin d'une donnée de BC.

Le précédent existant, `ISpacesHostLayout`, va dans **l'autre sens** — le BC
déclare, l'hôte implémente. Ne pas le prendre pour modèle.

**Injection** : un champ sur `AppState`, instancié dans `main.rs`. `app_menu.rs`
le lit comme il lit déjà `state.spaces.space_repository`.

### Une requête par rendu de menu, et c'est cohérent

`app_menu.rs` relit déjà le profil de membre à **chaque** rendu, volontairement
sans cache :

> « Le profil est relu à chaque rendu du menu, jamais mis en cache : une
> rétrogradation doit faire disparaître l'entrée au rafraîchissement suivant,
> pas à la reconnexion. »

Le même raisonnement tient : décocher l'option doit faire disparaître l'entrée
tout de suite. Le coût est une requête de plus par navigation HTMX, assumé pour
la même raison que la première.

## 5 · La garde à la création

Sur `create_match_report` : refus si la compétition **choisie dans le
formulaire** interdit le hors-calendrier.

Retirer les entrées de menu cache la fonction ; ça n'empêche pas d'appeler la
route à la main. Sans cette garde, le réglage est décoratif.

La garde porte sur la compétition choisie, pas sur l'espace : c'est le seul
endroit où la question a une réponse exacte.

## 6 · La phase 1 en lecture seule pour les non-admins

**Le GET et le POST restent ouverts à tous.** Un coach doit pouvoir saisir son
rapport ; bloquer la route l'enfermerait dehors, puisque c'est par là qu'il
entre depuis le Calendrier.

### Ce qu'il y a réellement à neutraliser : les deux équipes

```rust
pub struct UpdateMatchSelectionCommand {
    pub match_report_id: MatchReportId,
    pub home_team_id: TeamId,
    pub away_team_id: TeamId,
    pub confirmed_by: CoachId,
}
```

**La compétition, la saison et la journée ne sont modifiables par ce POST ni
aujourd'hui ni jamais** : le formulaire les envoie, le contrôleur ne les lit pas,
la commande ne les porte pas. Il n'y a qu'une chose à ignorer.

| | |
|---|---|
| `edit_match_report` (GET) | non-admin en compétition fermée → **vue texte** ; admin → les deux widgets |
| `update_match_selection` (POST) | non-admin → le contrôleur **ignore** `form.home_team_id` / `form.away_team_id` et bâtit la commande avec les valeurs déjà en base |

**La garde n'interdit rien, elle ignore.** Rien à afficher comme erreur, rien à
expliquer : un POST trafiqué produit exactement ce qu'aurait produit un POST
honnête. C'est plus solide qu'un refus, qu'il aurait fallu mettre en mots à
l'écran.

### Pourquoi ce n'est pas un `disabled` sur trois champs

`match-selection.html` ne contient **aucun champ**. C'est un `<form>` qui charge
deux widgets appartenant à d'autres BCs :

| | |
|---|---|
| `competition_widget` — compétition / saison / journée | BC `competitions` |
| `team_selection_widget` — équipes domicile et extérieur | BC `teams` |

La vue texte remplace les deux `hx-get` par les quatre noms en clair.
`MatchReportState::Draft` ne porte que des identifiants, mais
`ITeamDataPort` et `ICompetitionDataPort` existent déjà dans
`match_report/ports.rs` — rien à créer.

**Écarté :** passer un paramètre `readonly` aux deux widgets. Ça obligerait deux
autres BCs à porter un mode dont ils n'ont besoin nulle part ailleurs.

**Écarté aussi :** retirer le bouton et refuser le POST. Les sélecteurs
resteraient manipulables, le coach changerait la journée, cliquerait, et rien ne
se passerait — « sans effet » n'est pas « en lecture seule », et c'est le genre
d'écran qui fait croire à une panne.

## 7 · Le bouton du Calendrier admin, supprimé définitivement

`admin/schedule.html:15-16` mène à la saisie manuelle. Il part, **indépendamment
de l'option** : il ne sert à rien.

⚠️ **Ne pas emporter avec lui** `admin/widgets/schedule-round-detail.html:152-153`,
qui porte `from_pairing`. C'est le chemin d'entrée légitime dans le rapport d'une
rencontre du calendrier. Les deux sont dans les gabarits d'administration du
calendrier, à quelques fichiers d'écart, et les confondre casserait la saisie
pour tout le monde.

## Ce que la carte ne touche pas

**L'onglet Résultats et l'onglet Calendrier.** Le coach clique sa ligne de
calendrier, arrive sur la phase 1 du bon rapport, et la saisie continue.
`edit_match_report` aiguille seul : `Draft` → phase 1, `PreMatch` → step2,
`ReadyToPublish` → step5, `Cancelled` → 410, `Published` → 409.

**Les matchs hors calendrier déjà saisis.** Décocher n'efface rien et ne bloque
aucune saisie en cours ; seule la création de nouveaux est refusée.

**Le rapport orphelin de la carte 552.** Fermer la porte n'est pas réparer le
lien manquant entre un rapport manuel et son appariement.

## La symétrie

Option **cochée** → phase 1 modifiable par tous, trois entrées présentes,
création acceptée. C'est l'état de toutes les compétitions existantes, et il ne
change pas.

## Terminé quand

Sur une compétition dont l'option est décochée, un coach non administrateur :

- ne voit l'entrée « Saisir un match » **ni en desktop ni en mobile** ;
- ouvre le rapport d'une rencontre du calendrier, **voit** compétition, journée
  et équipes en texte, **ne peut pas** les changer, et **peut** enchaîner sur
  l'étape 2 ;
- reçoit un refus s'il appelle `create_match_report` à la main sur cette
  compétition.

Et un administrateur de la même compétition garde les deux widgets de la phase 1.

## Tests

**Unitaires**

| | |
|---|---|
| `CompetitionOptions` | une colonne `NULL` se désérialise en `autorise_hors_calendrier: true` |
| `update_general_options_use_case` | la sauvegarde ne touche que sa colonne — les quatre autres réglages sont relus inchangés |
| `construire` du contrôleur de phase 1 | pour un non-admin, la commande porte les équipes **en base**, pas celles du formulaire — falsifié en envoyant deux autres équipes |

**E2E** (`test_competition_hors_calendrier.py`, nouveau)

| | |
|---|---|
| l'entrée de menu disparaît | vérifiée en **desktop et en mobile** — deux markups, deux vérifications ; un seul `viewport` ne prouve rien |
| la phase 1 en lecture seule | le coach voit les noms, aucun `kreek-select` n'est présent, et le bouton d'enchaînement marche |
| le POST trafiqué ne change rien | envoyer deux autres équipes, relire en base : inchangées |
| la symétrie | option recochée, l'entrée revient |

**À ne pas oublier** : l'axe 8 de `check-arch` refuse un test e2e absent de
`tests/impact-map.toml`. Les BCs traversés sont `competitions`, `auth`, `teams`,
`spaces`, `references` et `match_report`.

## Checklist

- [x] La colonne, sa migration datée après le 2026-09-08, `find_options` / `save_options`
- [x] `CompetitionOptions` avec son `#[serde(default)]` à `true`, et son test
- [x] Le panneau « Réglages généraux », deuxième position, sur le modèle de `visibility_panel.rs`
- [x] Son use case, qui ne touche que sa colonne
- [x] `IHorsCalendrierPort` dans `src/web/ports.rs`, son adapter dans `src/infrastructure/web/`
- [x] Injecté dans `AppState` par `main.rs`
- [x] Les trois entrées conditionnelles — **desktop et mobile séparément**
- [x] Le commentaire de `peut_administrer` corrigé : il n'est plus le seul
- [x] La garde sur `create_match_report`
- [x] La vue texte de la phase 1, et les deux équipes ignorées au POST
- [x] Le bouton de `admin/schedule.html` retiré, **`from_pairing` intact**
- [x] L'entrée dans `tests/impact-map.toml`
- [x] `make lint`, `make check-arch`, `make test`, le nouvel e2e
