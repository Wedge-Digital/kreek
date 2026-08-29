# Le panneau de statistiques lit les vrais matchs

**Épic :** aucune · **Ordre :** 1 · **Dépend de :** rien
**Conception :** `docs/specs/statistiques-de-competition/README.md`
**Remplace :** la carte `13-mock-data-competition-detail`

## Objectif

Les quatre tableaux de l'onglet Statistiques affichent les chiffres de la
saison consultée, et non cinq coachs inventés identiques pour toutes les
compétitions.

## Le constat

`competition_detail.rs` sert de la donnée fictive **en production** :

```rust
top_tds: mock_top_tds(),            // « CoachAlpha », 18 TD, toujours
top_casualties: mock_top_casualties(),
flop_tds: mock_flop_tds(),
flop_casualties: mock_flop_casualties(),
```

Appelées aux deux sorties de `get_tab_stats` — le fragment sur `hx-request`, la
page pleine sinon.

## La donnée existe, et elle est chez `competitions`

`competition_match_display_proj` porte par match `home_score` / `away_score`
(les touchdowns), `home_casualties` / `away_casualties`, les identifiants et les
noms dénormalisés des deux équipes et de leurs coachs.

**Les quatre tableaux sont quatre tris d'une seule agrégation de cette table.**
Aucun port inter-BC, aucune colonne neuve, aucune projection à créer.

### La sémantique, vérifiée et non supposée

```rust
// match_report_published_listener.rs:84
let home_cas = count_casualties(&payload.home_actions);
```

`home_casualties` compte les blessures **infligées par** l'équipe à domicile.
« Encaissées » par elle, c'est `away_casualties`.

**L'inversion produirait quatre tableaux plausibles et faux** — les mêmes
équipes, les mêmes nombres, deux titres échangés. Rien ne la signalerait.

### Pourquoi pas `ranking_lines`

Elle porte pourtant `td_for`, `td_against`, `casualties` en cumulé. Mais elle
appartient à `ranking`, **elle n'a pas de `casualties_against`** — le quatrième
tableau n'y a aucune source — et elle ne couvre que les matchs entrés au
classement.

## La requête — une seule, quatre tris en Rust

```sql
-- io/repository/sql/stats/season_team_totals.sql
WITH par_equipe AS (
    SELECT home_team_id AS team_id, home_team_name AS team_name,
           home_coach_name AS coach_name,
           home_score AS td_for, away_score AS td_against,
           home_casualties AS cas_for, away_casualties AS cas_against
    FROM competition_match_display_proj
    WHERE season_id = $1 AND match_status = 'completed'
    UNION ALL
    SELECT away_team_id, away_team_name, away_coach_name,
           away_score, home_score, away_casualties, home_casualties
    FROM competition_match_display_proj
    WHERE season_id = $1 AND match_status = 'completed'
)
SELECT team_id,
       max(team_name)  AS "team_name!",
       max(coach_name) AS "coach_name!",
       COALESCE(sum(td_for),      0)::int AS "td_for!",
       COALESCE(sum(td_against),  0)::int AS "td_against!",
       COALESCE(sum(cas_for),     0)::int AS "cas_for!",
       COALESCE(sum(cas_against), 0)::int AS "cas_against!"
FROM par_equipe
GROUP BY team_id
```

**Une requête et non quatre** : les quatre tableaux viennent alors du même
instantané et sont cohérents entre eux. Quatre requêtes pourraient tomber de
part et d'autre d'une publication et afficher des totaux qui ne s'additionnent
pas.

**`UNION ALL`** : deux lignes identiques par le hasard des chiffres sont deux
matchs, pas un doublon.

**`COALESCE`** : les colonnes sont nullable. Une somme qui rencontre un `NULL`
ne vaut pas zéro, **elle vaut `NULL`** — et l'équipe disparaîtrait des quatre
tableaux sans un mot.

## Le contrat

```rust
// competitions/domain/match_day_repository_port.rs
pub struct TeamStatTotalsDto {
    pub team_id: String, pub team_name: String, pub coach_name: String,
    pub td_for: u32, pub td_against: u32, pub cas_for: u32, pub cas_against: u32,
}

async fn find_season_stat_totals(&self, season_id: &str)
    -> Result<Vec<TeamStatTotalsDto>, MatchDayRepositoryError>;
```

DTO de lecture : les primitives y sont autorisées, il ne porte aucun invariant.
Le port possède déjà cette table (`list_resultats`, `list_calendrier`,
`list_latest_completed_results`).

## Le builder

```rust
// competitions/io/web/stats_view.rs
pub struct StatTables {
    pub top_tds: Vec<StatRow>, pub top_casualties: Vec<StatRow>,
    pub flop_tds: Vec<StatRow>, pub flop_casualties: Vec<StatRow>,
}
pub fn build_stat_tables(totals: Vec<TeamStatTotalsDto>) -> StatTables;
```

`StatRow { rank, coach, team, value }` **existe déjà** et le gabarit l'attend :
rien à toucher côté vue.

**Une structure et non quatre `Vec` rendus séparément** : `full_page()` prend
aujourd'hui quatre `Vec<StatRow>` positionnels d'affilée, la configuration exacte
où deux s'intervertissent sans que le compilateur bronche — et où le seul
symptôme est un tableau titré « Top » qui montre des flops.

### Les cinq décisions, dans le builder

| | |
|---|---|
| **Cinq lignes** par tableau | `take(5)` |
| **Ex æquo : ordre arbitraire mais stable** | `sort_by(valeur DESC).then(team_id)` |
| **Global à la saison**, pas par poule | aucun filtre de groupe |
| **Seuls les `completed`** | dans la requête |
| **État vide explicite** | voir ci-dessous |

**« Arbitraire » ne veut pas dire « instable ».** Sans second critère, l'ordre
de deux équipes à 18 TD peut changer d'un rafraîchissement à l'autre — ce qui
ressemble à un défaut de calcul. On ne décide pas qui passe devant ; on décide
que **ce sera toujours le même**.

## L'état vide — un seul, pas quatre

Les quatre tableaux sortent de la même requête : si l'un est vide, les quatre le
sont. Il n'existe aucune saison avec des touchdowns et sans blessures.

Donc **un message unique au-dessus de la grille**, et non quatre en-têtes
surmontant quatre vides — quatre fois le même message donnerait l'impression de
quatre défauts.

> Aucun match n'a encore été publié dans cette saison.

## Ce qui disparaît

| Fonction | |
|---|---|
| les quatre `mock_*` de statistiques | supprimées |
| `mock_teams` | supprimée — **elle n'est déjà appelée nulle part** |

`mock_teams()` est du code mort : l'onglet Équipes est branché, et
`TeamsTabTemplate` ne porte plus que `app_routes`, `space_id` et `season_id`. La
carte 13 le décrivait encore comme du mock — c'est pour ça qu'elle est
remplacée plutôt que reprise.

## Tests

### Unitaires — `build_stat_tables`, sans base

| Test | Ce qu'il prouve |
|---|---|
| `le_top_td_classe_par_td_marques_decroissants` | le tri nominal |
| `le_flop_td_classe_par_td_encaisses_decroissants` | le flop est le pire, pas le meilleur |
| `les_blessures_pour_et_contre_ne_sont_pas_inversees` | **la sémantique du listener** |
| `les_rangs_partent_de_un_et_se_suivent` | même à ex æquo |
| `deux_ex_aequo_gardent_le_meme_ordre_a_deux_appels` | arbitraire, mais stable |
| `seules_cinq_lignes_sortent_sur_douze_equipes` | la borne |
| `moins_de_cinq_equipes_donne_moins_de_cinq_lignes` | pas de bourrage |
| `aucune_equipe_donne_quatre_tableaux_vides` | l'état vide |

`les_blessures_pour_et_contre_ne_sont_pas_inversees` se construit sur une équipe
qui **inflige beaucoup et encaisse peu** : une inversion la ferait passer de
première du Top à première du Flop.

### Intégration — vraie `PgPool`

| Test | Ce qu'il prouve |
|---|---|
| `un_match_in_progress_ne_compte_pas` | le filtre de statut |
| `un_match_d_une_autre_saison_ne_compte_pas` | le filtre de saison |
| `une_colonne_nulle_ne_fait_pas_disparaitre_l_equipe` | le `COALESCE` |
| `une_equipe_compte_ses_matchs_a_domicile_et_a_l_exterieur` | le `UNION ALL` |

Le dernier est le seul qui attrape une moitié d'`UNION` oubliée — un défaut qui
donne des totaux **exactement divisés par deux** pour les équipes qui jouent
autant des deux côtés, donc plausibles.

## Ce que la carte ne fait pas

- **Aucune statistique de joueur** — la donnée est dans `players`, autre BC,
  autre écran.
- **Aucun filtre** par poule, journée ou période.
- **Aucune consolidation des `fouls` et `completions`** : le panneau a quatre
  tableaux, en ajouter serait changer l'écran.

## Le point de surveillance

`ranking_lines` porte les mêmes nombres, de son côté. **Les deux peuvent
diverger** : un rapport qui alimente le classement sans que la projection
d'affichage suive donnerait des stats en désaccord avec le classement détaillé.
C'est le mode de panne exact de la carte 427 — un listener sur trois qui
abandonne en silence. Rien à faire ici, mais c'est à savoir le jour où l'écart
se voit.

## Checklist

- [ ] `sql/stats/season_team_totals.sql`
- [ ] `TeamStatTotalsDto` + `find_season_stat_totals` sur `IMatchDayRepository`
- [ ] `io/web/stats_view.rs` — `StatTables` et `build_stat_tables`
- [ ] `full_page()` prend `StatTables` au lieu de quatre `Vec` positionnels
- [ ] Les deux sorties de `get_tab_stats` branchées
- [ ] État vide dans `competition-tab-stats.html`, **un seul message**
- [ ] Les cinq `mock_*` supprimées, `mock_teams` comprise
- [ ] Les huit tests unitaires, les quatre d'intégration
- [ ] `make lint && make test && make check-arch`
