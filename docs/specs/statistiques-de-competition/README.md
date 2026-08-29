# Statistiques d'une compétition — brancher le panneau

**Gabarit :** `src/app/competitions/io/web/templates/competition-tab-stats.html`
**Carte périmée qu'elle remplace :** `13-mock-data-competition-detail`

## Pourquoi une spec courte et non le workflow feature

Le workflow en huit phases existe pour une **nouvelle page ou un nouveau
parcours**. Ici il n'y a ni écran à concevoir — le gabarit et son CSS sont finis
— ni règle métier, ni écriture, ni événement, ni domaine. Les phases 1, 2, 5 et
6 n'auraient rien à dire.

Ce document fond ce qui reste : la donnée, le contrat, le branchement.

## Ce que le panneau affiche

Quatre tableaux en grille 2×2, **cinq lignes chacun**, une ligne étant
`rang · coach · équipe · valeur` :

| Tableau | Sous-titre | Valeur |
|---|---|---|
| Top Touchdowns | marqués | TD pour |
| Top Blessures | infligées | blessures infligées |
| Flop Touchdowns | encaissés | TD contre |
| Flop Blessures | encaissées | blessures subies |

## La donnée existe, et elle appartient à `competitions`

`competition_match_display_proj` porte, **par match** :

```
season_id · match_status
home_team_id / home_team_name / home_coach_name · home_score · home_casualties
away_team_id / away_team_name / away_coach_name · away_score · away_casualties
```

**Les quatre tableaux sont quatre tris d'une seule agrégation de cette table.**
Pour une équipe, « pour » c'est sa colonne à domicile plus l'autre à
l'extérieur ; « contre » c'est le miroir.

Cette table est celle du BC qui sert l'onglet : **aucun port inter-BC, aucune
colonne neuve, aucune projection à créer.**

### La sémantique, vérifiée et non supposée

```rust
// match_report_published_listener.rs:84
let home_cas = count_casualties(&payload.home_actions);
```

`home_casualties` compte les blessures **infligées par** l'équipe à domicile.
« Encaissées » par elle, c'est donc `away_casualties`.

L'inversion produirait quatre tableaux **plausibles et faux** — les mêmes
équipes, les mêmes nombres, deux titres échangés. Rien ne la signalerait.

### La source concurrente, écartée

`ranking_lines` porte aussi `td_for`, `td_against`, `casualties`, `fouls`,
`completions` — cumulatives, donc la dernière ligne d'une équipe **est** son
total de saison. Une lecture, zéro agrégation. Tentant.

Trois raisons de ne pas y aller :

1. Elle appartient à `ranking` — il faudrait un port pour une donnée qu'on a
   déjà chez soi.
2. **Elle n'a pas de `casualties_against`** : le quatrième tableau n'a aucune
   source là-bas.
3. Elle ne couvre que les matchs entrés au classement.

**Le risque que ça laisse, et qu'il faut nommer** : deux tables porteront les
mêmes nombres et pourront diverger. Si un rapport alimente le classement sans
que la projection d'affichage suive, les stats et le classement détaillé se
contrediront. C'est le mode de panne exact de la carte 427 — un listener sur
trois qui abandonne en silence. Aucune parade ici ; c'est un point de
surveillance, pas un chantier.

## Les cinq décisions

| | Décision |
|---|---|
| 1 | **Cinq lignes** par tableau |
| 2 | **Ordre arbitraire** entre ex æquo — mais *stable*, voir ci-dessous |
| 3 | **Stats globales à la saison**, pas par poule |
| 4 | **Seuls les matchs `completed`** comptent |
| 5 | **État vide explicite** |

### « Arbitraire » ne veut pas dire « instable »

Sans second critère de tri, Postgres est **libre de rendre deux ordres
différents pour la même requête** — et il le fait, dès que le plan change. Deux
équipes à 18 TD permuteraient d'un rafraîchissement à l'autre, ce qui ressemble
à un défaut de calcul.

Le tri porte donc toujours un second critère : `ORDER BY <valeur> DESC,
team_id`. On ne décide pas *qui* passe devant — c'est bien arbitraire — mais on
décide que **ce sera toujours le même**.

### Décision 4 — sa conséquence à connaître

Un rapport **manuel en cours** est visible à l'onglet Résultats
(`match_status = 'in_progress'`) mais ne compte pas ici : ses colonnes de score
sont `NULL` tant qu'il n'est pas publié. Un organisateur peut donc voir un match
aux résultats sans le voir dans les stats. C'est cohérent — un match non publié
n'a pas de résultat acquis — mais ça se remarque.

### Décision 5 — un seul état vide, pas quatre

Les quatre tableaux sortent de **la même requête**. Si l'un est vide, les
quatre le sont : il n'existe aucun cas où une compétition aurait des
touchdowns sans blessures.

Donc **un message unique au-dessus de la grille**, et pas quatre en-têtes
surmontant quatre vides. Quatre fois le même message donnerait l'impression de
quatre défauts.

> Aucun match n'a encore été publié dans cette saison.

## La requête

Une seule, et quatre tris en Rust — plutôt que quatre requêtes.

```sql
-- sql/stats/season_team_totals.sql
WITH par_equipe AS (
    SELECT home_team_id   AS team_id,
           home_team_name AS team_name,
           home_coach_name AS coach_name,
           home_score      AS td_for,
           away_score      AS td_against,
           home_casualties AS cas_for,
           away_casualties AS cas_against
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

**Une requête et non quatre.** Outre l'aller-retour économisé, les quatre
tableaux sont alors garantis **cohérents entre eux** : ils viennent du même
instantané. Quatre requêtes pourraient tomber de part et d'autre d'une
publication de rapport et afficher des totaux qui ne s'additionnent pas.

**`UNION ALL` et non `UNION`** : deux lignes identiques par le hasard des
chiffres sont deux matchs, pas un doublon à dédupliquer.

**`COALESCE` sur les sommes** : les colonnes sont `INTEGER` nullable. Le filtre
`completed` devrait suffire à les remplir, mais un rapport publié dont le
listener aurait échoué à mi-chemin laisserait un `NULL` — et une somme qui
rencontre un `NULL` ne vaut pas zéro, elle **vaut `NULL`**, et l'équipe
disparaîtrait des quatre tableaux sans un mot.

**`max(team_name)`** : une équipe renommée en cours de saison a deux libellés
dans la projection. Le regroupement par `team_id` reste juste — les totaux sont
bons — mais l'étiquette est arbitraire parmi les variantes. C'est le même
compromis que l'onglet Résultats, où le nom est figé au moment du match.

## Le contrat

```rust
// competitions/domain/match_day_repository_port.rs — à côté des trois list_*
pub struct TeamStatTotalsDto {
    pub team_id:     String,
    pub team_name:   String,
    pub coach_name:  String,
    pub td_for:      u32,
    pub td_against:  u32,
    pub cas_for:     u32,
    pub cas_against: u32,
}

async fn find_season_stat_totals(
    &self,
    season_id: &str,
) -> Result<Vec<TeamStatTotalsDto>, MatchDayRepositoryError>;
```

Un DTO de lecture : **les primitives y sont autorisées** par le `CLAUDE.md`, ces
types ne portent aucun invariant.

Il rejoint `IMatchDayRepository`, qui possède déjà la table et porte
`list_resultats`, `list_calendrier`, `list_latest_completed_results`.

## Le view model, inchangé

```rust
pub struct StatRow { pub rank: u32, pub coach: String, pub team: String, pub value: u32 }
```

Il existe déjà, le gabarit l'attend, **rien à toucher**.

```rust
// competitions/io/web/stats_view.rs
pub fn build_stat_tables(totals: Vec<TeamStatTotalsDto>) -> StatTables;

pub struct StatTables {
    pub top_tds:          Vec<StatRow>,
    pub top_casualties:   Vec<StatRow>,
    pub flop_tds:         Vec<StatRow>,
    pub flop_casualties:  Vec<StatRow>,
}
```

### Pourquoi une structure et non quatre `Vec` rendus séparément

`full_page()` prend aujourd'hui **quatre `Vec<StatRow>` positionnels
d'affilée** :

```rust
full_page(pb, space_id, competition_id, season_id, "stats", false,
          mock_top_tds(), mock_top_casualties(), mock_flop_tds(), mock_flop_casualties())
```

Quatre arguments du même type, côte à côte : c'est la configuration exacte où
deux s'intervertissent sans que le compilateur bronche, et où le seul symptôme
est un tableau titré « Top » qui montre des flops. Une structure nomme chaque
place.

## Le branchement

`get_tab_stats` sert **deux fois** la même donnée — le fragment sur
`hx-request`, la page pleine sinon. Les deux chemins appellent le dépôt ; c'est
déjà le cas des quatre `mock_*`.

```
get_tab_stats
  ├─ hx-request → StatsTabTemplate  { les quatre Vec }
  └─ sinon      → full_page(…, tables)
```

### Ce qui disparaît

| Fonction | Sort |
|---|---|
| `mock_top_tds`, `mock_top_casualties`, `mock_flop_tds`, `mock_flop_casualties` | supprimées |
| `mock_teams` | supprimée — **elle n'est déjà appelée nulle part** |

`mock_teams()` est du code mort : l'onglet Équipes est branché depuis les cartes
du BC `teams`, et `TeamsTabTemplate` ne porte plus que `app_routes`, `space_id`
et `season_id`. La carte 13 le décrivait encore comme du mock.

## Tests

### Unitaires — sur `build_stat_tables`, sans base

| Test | Ce qu'il prouve |
|---|---|
| `le_top_td_classe_par_td_marques_decroissants` | le tri nominal |
| `le_flop_td_classe_par_td_encaisses_decroissants` | le flop est bien le pire, pas le meilleur |
| `les_blessures_pour_et_contre_ne_sont_pas_inversees` | **la sémantique du listener** |
| `les_rangs_partent_de_un_et_se_suivent` | même à ex æquo |
| `deux_ex_aequo_gardent_le_meme_ordre_a_deux_appels` | arbitraire, mais stable |
| `seules_cinq_lignes_sortent_sur_douze_equipes` | la borne |
| `moins_de_cinq_equipes_donne_moins_de_cinq_lignes` | pas de bourrage |
| `aucune_equipe_donne_quatre_tableaux_vides` | l'état vide |

`les_blessures_pour_et_contre_ne_sont_pas_inversees` est celui qui compte : il
se construit sur une équipe qui **inflige beaucoup et encaisse peu**, de sorte
qu'une inversion la ferait passer de première du Top à première du Flop.

### Intégration — sur une vraie `PgPool`

| Test | Ce qu'il prouve |
|---|---|
| `un_match_in_progress_ne_compte_pas` | décision 4 |
| `un_match_d_une_autre_saison_ne_compte_pas` | le filtre `season_id` |
| `une_colonne_nulle_ne_fait_pas_disparaitre_l_equipe` | le `COALESCE` |
| `une_equipe_compte_ses_matchs_a_domicile_et_a_l_exterieur` | le `UNION ALL` |

Le dernier est le seul qui puisse attraper une moitié d'`UNION` oubliée — un
défaut qui donne des totaux **exactement divisés par deux** pour les équipes
qui jouent autant des deux côtés, donc plausibles.

### E2E

`tests/e2e/test_competition_stats.py`.

| Scénario | Ce qu'il prouve |
|---|---|
| `test_l_onglet_stats_affiche_les_quatre_tableaux` | le chemin heureux |
| `test_les_valeurs_correspondent_aux_rapports_publies` | le bout en bout |
| `test_une_saison_sans_match_affiche_l_etat_vide` | décision 5 |

**Aucun `sleep`** : l'onglet arrive par swap HTMX, donc `cliquer_quand_cable`
pour tout clic qui le suit.

## Ce que ce chantier ne fait pas

- **Aucune statistique de joueur** — meilleur marqueur, joueur le plus blessé.
  La donnée existe dans `players`, mais c'est un autre BC et un autre écran.
- **Aucun filtre** par poule, par journée, par période.
- **Aucune évolution dans le temps** — pas de courbe, pas de comparaison.
- **Aucune consolidation des `fouls` et `completions`**, pourtant présentes
  dans `ranking_lines` : le panneau n'a que quatre tableaux, et les ajouter
  serait changer l'écran.

## Les cartes

| # | Carte | Dépend de |
|---|---|---|
| 474 | Le panneau de statistiques lit les vrais matchs | rien |
| 475 | Les tests e2e du panneau de statistiques | 474 |
