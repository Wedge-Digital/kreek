# Les matchs d'une équipe — brancher l'onglet

**Gabarit à réemployer :** `competitions/io/web/templates/competition-tab-resultats.html`
**Onglet inerte :** `teams-team-detail.html:147` — `<div class="tab">Matchs</div>`
**Carte périmée qu'elle remplace :** `match-02-team-matches-tab` (épic E06)

## Pourquoi une spec courte

Pas d'écriture, pas de domaine, pas d'événement, pas de règle métier. Le bloc de
match existe et il est générique. Reste la requête, le tri, une pastille et un
contrôle d'accès.

Mais ce n'est pas un copier-coller nu : quatre choses ne se transposent pas, et
c'est ce document qui les nomme.

## Ce qui se réemploie tel quel

| Quoi | Où |
|---|---|
| Le bloc de match — logos, noms, roster · coach, score TD, blessures, badge « en cours », lien rapport | `competition-tab-resultats.html`, 60 lignes |
| `MatchResultatVm` — 17 champs, tous symétriques domicile/extérieur | `resultats_view.rs:11` |
| La donnée, **dénormalisée** : noms, logos, initiales, rosters, coachs | `competition_match_display_proj` |
| Le contrôle d'accès | `compute_authorization()` — **verbatim**, voir plus bas |

La table appartient à `competitions`. **C'est donc `competitions` qui sert le
fragment**, et la fiche d'équipe le compose par `hx-get`.

C'est le patron déjà en place à la ligne 153 de la fiche :

```rust
players_widget_url: app_routes.players.players_by_team_widget(space_id, &team.id.to_string()),
```

### La carte `match-02` se trompait de BC

> **Dépend de :** BC `match_report` (non encore créé)
> **Checklist :** Modèle de persistance des matchs dans BC `match_report`

Le BC existe, et **il n'y a rien à modéliser** : la projection porte déjà tout
ce que le bloc affiche. Même erreur que la carte 13 sur les statistiques —
demander un ticket de modélisation pour une donnée qui est là depuis l'origine.

## Les quatre décisions

| | Décision |
|---|---|
| 1 | **Saison courante seulement** |
| 2 | **Les matchs à venir sont inclus** |
| 3 | **Une pastille V/N/D** du point de vue de l'équipe consultée |
| 4 | **Le coach de cette équipe et les admins** voient le lien vers le rapport |

### Décision 1 — ce qu'elle évite

Une équipe **peut jouer plusieurs saisons** :

```rust
// teams/domain/team.rs:69
TeamEnrolled { competition_id, competition_name, season_id, season_name },
```

Sans cette décision, `WHERE home_team_id = $1 OR away_team_id = $1` ramènerait
tout l'historique, et trois choses casseraient : « Journée 1 » reviendrait à
chaque saison en groupes homonymes, le curseur `round_position` se répéterait
d'une saison à l'autre en sautant des matchs, et il faudrait un en-tête par
compétition — que la projection ne sait pas nommer, faute de `competition_id`.

**La saison courante fait disparaître les trois d'un coup.**

### Décision 2 — l'ordre, qui n'est pas celui de la compétition

L'onglet compétition trie `round_position DESC` et exclut `upcoming`. Reprendre
ce tri en incluant les matchs à venir mettrait **le match le plus lointain en
tête** et enterrerait le prochain au milieu de la liste.

L'ordre est donc celui d'une chronologie centrée sur maintenant :

```
1. le match en cours de saisie, s'il y en a un
2. les matchs à venir, du plus proche au plus lointain   (round_position ASC)
3. les matchs joués, du plus récent au plus ancien        (round_position DESC)
```

Sur une fiche d'équipe, « mon prochain match » est ce qu'un coach vient
chercher ; le mettre en tête est le seul ordre qui le serve.

### Décision 2 bis — pas de pagination

L'onglet compétition pagine par `scroll-sentinel` et `LIMIT 500` : une saison,
c'est des centaines de matchs. **Une équipe en joue dix à quinze.** Le
sentinelle et le curseur disparaissent — moins de code, et un curseur en moins
à faire mentir.

### Décision 3 — la pastille se dérive, elle ne se requête pas

```rust
let is_home = row.home_team_id == team_id;
let (pour, contre) = if is_home { (home_score, away_score) } else { (away_score, home_score) };
```

Aucune requête de plus, aucune donnée de plus. `Option<MatchOutcome>` sur le
VM : `None` sur l'onglet compétition, où la question n'a pas de sens — il n'y a
pas d'équipe de référence.

**`Option` et non un booléen ni une chaîne vide** : « pas d'équipe de référence »
et « match nul » sont deux choses, et un `String::new()` les confondrait.

Un match non joué n'a pas d'issue : `None` aussi.

### Décision 4 — `compute_authorization` convient **verbatim**

C'est le point le plus heureux de ce chantier. La fonction existante calcule :

```
is_admin      = admin d'espace ou admin de la compétition
my_team_ids   = les équipes de la saison dont l'utilisateur est le coach
allows(h, a)  = is_admin || h ∈ my_team_ids || a ∈ my_team_ids
```

Appliquée aux matchs d'une seule équipe, elle rend exactement la règle demandée :

| Qui consulte la fiche de l'équipe A | Lien vers le rapport |
|---|---|
| admin d'espace ou de la compétition | **oui**, sur tous |
| le coach de A | **oui**, sur tous |
| le coach de B, sur un match A–B | **oui** — il a joué ce match |
| le coach de B, sur un match A–C | non |

La troisième ligne est meilleure qu'une règle écrite à la main : un coach garde
l'accès aux rapports de **ses** matchs, où qu'il les regarde. Une règle « le
coach de cette équipe et lui seul » le lui retirerait sur la fiche de son
adversaire alors qu'il l'a depuis l'onglet Résultats — une incohérence entre
deux écrans qui montrent le même match.

**Zéro ligne de contrôle d'accès à écrire.**

## Le piège de sécurité — la compétition ne se prend pas dans l'URL

`compute_authorization` a besoin du `competition_id` et du `season_id`. La
tentation est de les baker dans l'URL du widget, comme la règle 4 des widgets le
demande pour les paramètres contextuels.

**Il ne faut pas.** `is_comp_admin` est calculé en consultant la liste des
admins de la compétition *passée en paramètre* :

```
utilisateur admin de la compétition X
→ ouvre la fiche d'une équipe de la compétition Y
→ force ?competition_id=X
→ is_comp_admin = vrai
→ liens vers les rapports de Y
```

`space_scope` ne l'attrape pas : X et Y sont dans le même espace, les deux
résolvent. **Le `competition_id` doit être résolu côté serveur depuis le
`team_id`**, qui est le seul identifiant du chemin, et que `TeamSpaceOwnership`
scope déjà.

D'où une méthode de plus sur le port existant :

```rust
// competitions/ports.rs — à côté de find_enrolled_teams et find_team_names
async fn find_team_enrollment(&self, team_id: &str)
    -> Result<Option<TeamEnrollmentDto>, String>;

pub struct TeamEnrollmentDto {
    pub competition_id: String,
    pub season_id:      String,
}
```

## Ce qui ne se transpose pas — le groupement

L'onglet compétition groupe par journée : un en-tête « Journée 3 · 6 matchs »,
puis six blocs.

**Une équipe joue un match par journée.** Reprendre le groupement donnerait
quinze groupes d'un match, chacun titré « 1 match ». Absurde.

La liste devient donc **plate**, et le libellé de journée passe *dans* le bloc :

```rust
pub round_label: Option<String>,   // « Journée 3 » — None sur l'onglet compétition
```

`None` là-bas parce que l'en-tête de groupe le dit déjà. Le partagé rend le
libellé s'il y en a un.

## Ce qui ne se transpose pas — le CSS

Les 93 règles `.match-widget`, `.matches-list`, `.match-side`, `.match-score`…
vivent dans `pages/competition-detail.css`, **toutes préfixées
`.competition-detail`** :

```css
.competition-detail .match-widget,
.competition-detail.match-widget { … }
```

Sur la fiche d'équipe elles ne s'appliqueraient **pas du tout** — le bloc
s'afficherait nu. Et retirer le préfixe ferait crier `debordements.py` :

> ce sélecteur trouve-t-il du markup sur une page qui ne chargeait pas sa
> feuille ?

**Elles deviennent `components/match-widget.css`.** Les feuilles de
`components/` sont globales par construction pour ce contrôle. C'est le même
geste que les teintes de compétence — et c'est la partie la plus risquée du
chantier, parce qu'elle touche une page qui marche.

## Le gabarit partagé

```
competitions/io/web/templates/
├── match-widget.html                  ← extrait, prend `m: MatchResultatVm`
├── competition-tab-resultats.html     ← l'inclut, garde ses groupes et son curseur
└── team-matches-widget.html           ← l'inclut, liste plate
```

L'extraction est un **copier-coller** (règle 5 du `CLAUDE.md`), pas une
réécriture : 60 lignes qui marchent, avec leurs `{% if let Some %}` et leurs
`unwrap_or(0)`.

## La requête

```sql
-- sql/match_days/list_team_matches.sql
SELECT
    pairing_id, round_id, round_name, round_position,
    round_date_start, round_date_end, round_day_type,
    home_team_id, home_team_name, home_roster_name, home_coach_name, home_logo_url, home_initials,
    away_team_id, away_team_name, away_roster_name, away_coach_name, away_logo_url, away_initials,
    match_status, home_score, away_score, home_casualties, away_casualties, match_report_url
FROM competition_match_display_proj
WHERE season_id = $1
  AND (home_team_id = $2 OR away_team_id = $2)
ORDER BY
    CASE match_status WHEN 'in_progress' THEN 0 WHEN 'upcoming' THEN 1 ELSE 2 END,
    CASE WHEN match_status = 'upcoming' THEN round_position END ASC,
    round_position DESC
```

Le `SELECT` est celui de `list_resultats.sql`, **inchangé** : c'est ce qui
garantit que le même VM se construit sans adaptation.

**Pas de filtre de statut** — les trois sont voulus (décision 2), et c'est le
`ORDER BY` qui les range.

**Le double `CASE`** exprime la chronologie centrée : d'abord le rang de
famille, puis l'ordre *croissant* à l'intérieur des seuls « à venir », puis
décroissant pour le reste. Deux tris opposés dans une requête, ce qui se lit mal
et mérite ce commentaire dans le fichier.

## Le contrôle d'accès en une ligne de plus

Le widget ne connaît que `team_id`. Il résout l'inscription, puis appelle la
fonction existante :

```rust
let enr = team_info_port.find_team_enrollment(team_id).await?;      // ← neuf
let authz = compute_authorization(state, user, space_id,
                                  &CompetitionId::try_new(&enr.competition_id)?,
                                  &enr.season_id).await;            // ← existant
```

Une équipe sans inscription — brouillon non soumis — rend une liste vide, pas
une erreur : elle n'a joué aucun match, ce qui est vrai.

## L'état vide

> Aucun match pour le moment.

Le même que l'onglet compétition, au mot près. Il couvre les deux cas qui s'y
présentent : l'équipe qui n'a pas encore de calendrier, et le brouillon non
inscrit.

## Tests

### Unitaires — sur le builder, sans base

| Test | Ce qu'il prouve |
|---|---|
| `une_victoire_a_domicile_donne_la_pastille_v` | décision 3 |
| `une_victoire_a_l_exterieur_donne_aussi_la_pastille_v` | **le sens de `is_home`** |
| `un_score_egal_donne_la_pastille_n` | le nul |
| `un_match_a_venir_n_a_pas_de_pastille` | `None`, pas « N » |
| `le_libelle_de_journee_accompagne_chaque_match` | la liste plate |
| `l_onglet_competition_ne_rend_aucune_pastille` | non-régression |

`une_victoire_a_l_exterieur_donne_aussi_la_pastille_v` est celui qui compte :
une inversion de `is_home` donne une pastille **fausse une fois sur deux**, ce
qui ressemble à un défaut de données et non à un défaut de code.

### Intégration — vraie `PgPool`

| Test | Ce qu'il prouve |
|---|---|
| `les_matchs_d_une_autre_saison_ne_remontent_pas` | décision 1 |
| `un_match_a_domicile_et_un_a_l_exterieur_remontent_tous_deux` | le `OR` |
| `le_prochain_match_est_en_tete` | l'ordre |
| `un_match_en_cours_de_saisie_passe_devant_les_a_venir` | l'ordre, deuxième moitié |

### E2E

`tests/e2e/test_team_matches.py`.

| Scénario | Ce qu'il prouve |
|---|---|
| `test_l_onglet_matchs_liste_les_matchs_de_l_equipe` | le chemin heureux |
| `test_le_coach_de_l_equipe_peut_ouvrir_le_rapport` | décision 4 |
| `test_un_visiteur_ne_voit_pas_le_lien_du_rapport` | décision 4, l'autre moitié |
| `test_une_equipe_sans_match_affiche_l_etat_vide` | l'état vide |
| `test_le_bloc_de_match_reste_correct_sur_la_page_competition` | **la non-régression CSS** |

Le dernier n'est pas une politesse : l'extraction des 93 règles touche une page
qui marche aujourd'hui, et c'est le seul risque réel du chantier.

## Ce que ce chantier ne fait pas

- **Aucun historique multi-saison** (décision 1).
- **Aucun bilan V/N/D agrégé** — c'est la carte `match-01` de la même épic. Sa
  source est désormais connue : la même requête, comptée.
- **Aucun filtre**, aucun tri choisi par l'utilisateur.
- **Aucune statistique de match** au-delà de ce que le bloc affiche déjà.

## Les cartes

| # | Carte | Dépend de |
|---|---|---|
| 476 | Le bloc de match devient un composant | rien |
| 477 | L'onglet Matchs d'une équipe | 476, **434** |
| 478 | Les tests e2e de l'onglet Matchs | 477 |

La **434** — « la fiche équipe accueille des onglets » — pose le mécanisme
d'aiguillage. L'onglet Matchs est aujourd'hui un `<div class="tab">` sans
`hx-get` ni handler ; il n'a nulle part où se brancher avant elle.

Son mécanisme est spécifié à part, dans
**`docs/specs/fiche-equipe-onglets/README.md`** — il servait la seule trésorerie
quand il a été conçu, il sert maintenant trois onglets dont un servi par un
autre BC. C'est là que se trouve la forme de la coquille, et la règle qui décide
quand un onglet se câble.
