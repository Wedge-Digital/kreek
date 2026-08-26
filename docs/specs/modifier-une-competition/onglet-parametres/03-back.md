# Onglet Paramètres · Phase 3 : architecture back

**Phase 2** : `02-front.md` · **Maquette** :
`assets/rawpages/html/app-competition-admin-modification.html`

## Ce qui est déjà écrit

Quatre des cinq enregistrements ont déjà leur use case. Le magicien de création
les appelle depuis ses étapes ; l'onglet Paramètres les rappelle sur une
compétition démarrée, **sans les modifier**.

| Panneau | Use case | Écrit dans |
|---|---|---|
| Informations générales | `update_draft_competition` + `save_competition_rules` | `competitions.name/logo` et `competition_seasons.name` |
| Points de classement | `save_competition_rules` | `competition_seasons.rules → ranking_rules` |
| Poules | `save_competition_structure` | `competition_seasons.structure → ranking_group` |
| Tiers & coups de pouce | `save_competition_rules` | `competition_seasons.rules → tiers` |
| Visibilité | `save_competition_invitations` | `competition_seasons.invitations` |

**Aucun n'a de garde de statut.** `update_draft_competition` ne vérifie rien
d'autre que l'unicité du nom dans l'espace ; `save_competition_rules` ne vérifie
que l'unicité des rosters et l'existence des codes de départage. Le mot `draft`
de `update_draft_competition` est un reste de son premier appelant, pas une
condition — il vaut mieux le renommer `update_competition_identity` en passant,
le magicien suivra.

Le seul travail vraiment neuf est le **recalcul du classement**, et la façon
dont un POST de `competitions` le déclenche.

## Le piège : cinq panneaux, trois documents JSONB

Les cinq panneaux n'écrivent pas dans cinq endroits. Ils écrivent dans trois
colonnes JSONB, et **chaque écriture remplace la colonne entière** :

| Colonne | Ce qu'elle porte | Qui l'écrit |
|---|---|---|
| `rules` | `ranking_rules` **+** `tiers` | deux panneaux |
| `structure` | `ranking_group` **+** `play_offs_phase` **+** `schedule` | un panneau, trois contenus |
| `invitations` | `access_mode`, `requires_validation`, `invited_coaches`, `max_participants`, `registration_deadline` | un panneau, deux champs sur cinq |

**Règle qui vaut pour les cinq POST : relire, remplacer la partie éditée,
réécrire le tout.** Un POST qui construirait un `CompetitionStructure` neuf à
partir du seul formulaire des poules effacerait le calendrier de la saison.

Trois oublis possibles, tous silencieux, tous à couvrir par un test :

- le panneau des poules réécrit `structure` sans relire `schedule` → le
  calendrier disparaît ;
- le panneau de visibilité réécrit `invitations` sans relire `invited_coaches`
  → les coachs invités disparaissent ;
- `save_rules(season_id, **season_name**, rules)` réécrit aussi le **nom de la
  saison** : les panneaux Classement et Tiers doivent lui repasser le nom
  courant, sinon ils le vident.

De même, `update_draft_competition` porte `admin_ids` : le panneau des
informations générales, qui n'édite pas les administrateurs, doit relire la
liste courante et la repasser telle quelle.

**Concurrence assumée.** Deux administrateurs enregistrant deux panneaux à la
même seconde : le dernier écrit gagne, et il peut écraser l'autre partie du même
JSONB. On ne pose pas de verrou optimiste — l'usage est à un administrateur à la
fois, et le coût d'un jeton de version sur trois colonnes ne se justifie pas
ici. C'est un choix, pas un oubli.

## Le recalcul du classement

### `ranking` sait recalculer seul — sans relire un seul rapport de match

C'est le point qui décide de tout le reste. Une ligne de `ranking_lines` est
**cumulative** : elle porte les totaux de l'équipe après le match, pas les
statistiques du match. On pourrait croire qu'il faut donc redemander les
statistiques à `match_report`. Il n'en est rien : elles sont récupérables par
**différence de deux lignes consécutives** de la même équipe.

| Champ de `MatchStats` | Se retrouve par |
|---|---|
| `own_td` | `td_for(n) − td_for(n−1)` |
| `opponent_td` | `td_against(n) − td_against(n−1)` |
| `casualties_inflicted` | `casualties(n) − casualties(n−1)` |
| `fouls` | `fouls(n) − fouls(n−1)` |
| `completions` | `completions(n) − completions(n−1)` |

Le **résultat** du match n'a pas à être lu : `record_match` le redérive des deux
scores (`derive_outcome`). Les colonnes `wins` / `draws` / `losses` ne servent
donc pas au rejeu — elles en sont un produit, et le recalcul les reconstruit.

Conséquence : **aucun nouveau port, aucun appel à `match_report`.** Le BC
`ranking` est autosuffisant pour se rejouer, et le recalcul ne peut pas diverger
d'un rapport modifié entre-temps, puisqu'il ne relit rien.

### La fonction inverse vit dans le domaine

```rust
// ranking/domain/ranking_line.rs
impl RankingLine {
    /// L'inverse exact de `record_match` : retrouve les statistiques du match
    /// qui a fait passer les cumuls de `previous` à `current`.
    pub fn stats_between(previous: Option<&CumulativeTotals>, current: &RankingLine) -> MatchStats
}
```

Elle est dans le domaine et non dans le use case parce qu'elle est l'inverse de
`record_match` : **les deux doivent être modifiées ensemble**, et un champ
ajouté à `MatchStats` sans être ajouté ici produirait un recalcul qui perd cette
statistique — silencieusement, puisque la ligne resterait bien formée.

Le test qui referme le piège est une propriété :
`stats_between(record_match(p, ctx, s, r)) == s` pour toute statistique.

### Le use case

```rust
// ranking/use_cases/recompute_season_ranking_use_case.rs
pub async fn execute(
    season_id: &SeasonId,
    repo: &dyn IRankingRepository,
    competition_port: &dyn IRankingCompetitionPort,
) -> Result<RecomputeReport, RecomputeSeasonRankingError>
```

1. lire **toutes** les lignes de la saison, dans leur ordre d'origine ;
2. les grouper par équipe, et différencier chaque ligne avec la précédente pour
   retrouver `MatchStats` et `MatchContext` ;
3. lire le barème courant par `IRankingCompetitionPort::find_ranking_rules` —
   celui que `competitions` vient d'enregistrer ;
4. rejouer `RankingLine::record_match` depuis zéro, dans l'ordre, par équipe ;
5. remplacer les lignes de la saison **en une seule transaction**.

**L'ordre d'origine est celui du cumul** : `recorded_at` croissant, départagé par
`match_report_id` pour rester déterministe quand deux lignes partagent l'horodatage.

**Idempotent** : rejoué à barème inchangé, il réécrit exactement les mêmes
lignes. C'est le filet — et le second test unitaire.

### Deux méthodes de dépôt à ajouter

`IRankingRepository` n'expose aujourd'hui que les *dernières* lignes
(`find_latest_line`, `find_latest_lines_for_season`) et une suppression par
match. Il lui faut :

```rust
/// Toutes les lignes de la saison, dans l'ordre du cumul.
async fn find_all_lines_for_season(&self, season_id: &str)
    -> Result<Vec<RankingLineRow>, RankingRepositoryError>;

/// Remplace en bloc les lignes de la saison — suppression et insertion dans
/// la même transaction. Un recalcul interrompu ne laisse pas un classement
/// à moitié rejoué.
async fn replace_lines_for_season(&self, season_id: &str, lines: &[RankingLine])
    -> Result<(), RankingRepositoryError>;
```

`replace_lines_for_season` et non un `delete` suivi d'`insert_lines` : la
règle des projections vaut ici aussi — deux transactions, et l'échec de la
seconde laisse la saison **sans classement du tout**.

### Comment un POST de `competitions` déclenche `ranking`

C'est la question restée ouverte en phase 2. Le recalcul est **synchrone au
POST** (décidé), et `ranking_lines` appartient à `ranking` : il faut donc un
chemin de `competitions` vers `ranking` qui ne soit pas un import croisé.

| Voie | Ce qu'elle donne | Verdict |
|---|---|---|
| **App event** `RankingRulesChanged`, écouté par `ranking` | découplé, conforme à la doctrine « propagation d'effet → app event » | **écartée** : asynchrone, l'écran ne peut pas confirmer que c'est fait — ce que la décision « synchrone » exclut |
| **Second POST enchaîné par le front** vers une route de `ranking` | synchrone, aucun couplage back | **écartée** : l'orchestration part dans le navigateur. Onglet fermé entre les deux requêtes, et le barème est enregistré alors que le classement ne l'a pas suivi — sans trace |
| **Port de commande + adapter** | synchrone, orchestré côté serveur, `check-arch` satisfait | **retenue** |

```rust
// competitions/ports.rs — le BC décrit son besoin, pas la solution
#[async_trait]
pub trait IRankingRecomputePort: Send + Sync {
    /// Rejoue le classement de la saison avec le barème courant.
    async fn recompute_season(&self, season_id: &str) -> Result<(), String>;
}
```

```
src/infrastructure/competitions/ranking_recompute_adapter.rs
    → appelle recompute_season_ranking_use_case
```

**Ce que cette voie coûte, dit franchement** : c'est le premier port de
`competitions` qui *ordonne* au lieu de *demander* — les sept autres sont des
lectures. Le `CLAUDE.md` range la propagation d'effet du côté des app events, et
on s'en écarte ici pour une raison précise : **l'écran doit confirmer**. La
règle documentée oppose consultation et propagation ; ce cas est un troisième
type, la commande synchrone entre BCs. Si un second cas apparaît, la règle
mérite d'être complétée plutôt que contournée une fois de plus.

**Le sens de la dépendance reste sain** : `competitions` ignore toujours
`ranking_lines`, et `ranking` continue de lire le barème par son propre port.
Seul `infrastructure/competitions/` connaît les deux.

### Ce que le POST du barème fait, dans l'ordre

```
POST admin_settings_ranking
  ├─ save_competition_rules   (barème + nom de saison + tiers relus)
  ├─ ranking_recompute_port.recompute_season(season_id)
  └─ rendu du widget, avec le nombre de matchs rejoués
```

Le recalcul **après** l'enregistrement, jamais l'inverse : il lit le barème par
le port, donc il doit lire le nouveau. Et si le recalcul échoue, le barème reste
enregistré — l'écran le dit, et le rejeu est reproposable, puisqu'il est
idempotent.

**Le retrait d'une poule ne déclenche aucun recalcul** : `ranking_lines` ne
porte pas de colonne de poule (établi en phase 2), le regroupement est un filtre
d'affichage sur l'assignation courante. Le classement n'a rien à rejouer.

## Le nouvel onglet dans l'aiguillage

`admin_page.rs` aiguille par `match active_tab`. Trois changements :

- **branche `settings`** ajoutée, qui rend l'onglet d'assemblage (cinq
  conteneurs `hx-get`, aucun calcul) ;
- **branches `dashboard` et `results` retirées** ;
- **le défaut `_` devient `summary`** — il rend aujourd'hui le tableau de bord.
  `admin_page()` passe `"summary"` au lieu de `"dashboard"`.

Ce que la suppression des deux onglets emporte, vérifié :

| Fichier | Autre consommateur ? |
|---|---|
| `io/web/admin/dashboard.rs` | aucun |
| `io/web/admin/results_tab.rs` | aucun |
| `use_cases/admin/dashboard_query.rs` | `dashboard.rs` et `admin_page.rs` seuls |
| `templates/admin/dashboard.html`, `admin/results.html` | aucun |
| `pages/competition-admin-dashboard.css` | entièrement portée par `.competition-admin-dashboard` — meurt avec l'onglet, à retirer de `css_bundle.rs` |
| routes `COMPETITION_ADMIN_DASHBOARD` / `_RESULTS` + `admin_dashboard()` / `admin_results()` | seulement `admin-page.html`, dont les deux onglets partent |

`io/web/resultats_view.rs` **reste** : il sert aussi les onglets publics
Calendrier et Résultats.

## Les cinq routes de l'onglet

Une paire GET/POST par widget, toutes sous le préfixe admin existant, toutes
gardées par `require_admin_access`.

```
GET|POST /app/{space_id}/competitions/{competition_id}/{season_id}/admin/settings/general
                                                                          /ranking
                                                                          /pools
                                                                          /tiers
                                                                          /visibility
GET      …/admin/settings          ← l'onglet d'assemblage
```

**`require_admin_access` sur chacune, GET compris.** Le commentaire du fichier
le dit déjà : sans ce contrôle sur le chemin htmx, le changement d'onglet
contourne l'autorisation.

Et il répond à la question laissée ouverte en phase 2 — *tous les
administrateurs ont-ils accès à tous les panneaux ?* **Oui.** L'autorisation
est posée à l'entrée de l'administration, pas par panneau : admin d'espace ou
admin de compétition ouvre les cinq. Ajouter une granularité ici créerait un
second modèle de droits pour un seul écran.

## Ce que le back ne fournit pas

- **Aucun endpoint pour les administrateurs** : le panneau est un affichage.
- **Aucune suppression, aucune réinitialisation.** La maquette portait une
  « zone de danger » — *Réinitialiser la saison*, *Supprimer la saison* — elle
  en a été retirée. **La direction retenue est l'archivage** : à terme, une
  saison archivée sort des affichages par un filtre, et rien n'est détruit.

  Ce qu'une vraie suppression aurait demandé, et qui justifie de ne pas la
  faire : dix tables portent `season_id` — `competition_groups`,
  `competition_match_days`, `competition_match_display_proj`,
  `competition_notification_deliveries`, `competition_seasons`,
  `competitions_members`, `match_report_proj`, `ranking_lines`, `team_drafts`,
  `team_proj` — réparties sur quatre BCs, **et les trois flux d'événements n'en
  portent pas**. `team_event_store`, `players_events` et
  `match_report_event_store` ne connaissent que leur agrégat : les atteindre
  imposerait de passer par les projections pour retrouver quoi détruire, en se
  servant d'un dérivé rebuildable comme index de la destruction.

  *Réinitialiser* n'est pas la version douce du même bouton : effacer les
  résultats en gardant les équipes leur laisserait les SPP, les blessures, les
  valeurs et les trésoreries gagnés dans des matchs qui n'existent plus.
- **Aucun réglage de roster, de budget ni d'XP de départ** (phase 2).

## Points de vigilance pour l'implémentation

- La maquette porte deux `style=` en ligne (titres de section du panneau
  Classement, largeur du champ « Places max »). Les templates n'en admettent
  aucun — à passer en classes.
- Le rejeu tourne sur toutes les lignes de la saison : une saison de 20 équipes
  et 15 journées fait 300 lignes. C'est un `find`, une boucle et un `insert` en
  bloc — l'ordre de grandeur ne pose pas de question. Si une ligue atteignait
  un volume qui rend le POST perceptible, c'est la forme synchrone qu'il
  faudrait revoir, pas l'algorithme.
- **La différence se calcule en `u32`, les scores se stockent en `u8`.**
  `td_for` est un cumul `u32`, `MatchScore` est un `MatchScore(pub u8)` nu —
  sans constructeur intelligent, donc sans garde-fou. Un `as u8` sur un écart
  aberrant **replierait** la valeur en silence : la conversion passe par
  `u8::try_from`, et son échec arrête le recalcul. Seule une corruption de
  lignes peut le produire, et c'est précisément le cas qu'il ne faut pas
  maquiller en score plausible.
