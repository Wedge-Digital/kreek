# L'onglet Matchs d'une équipe

**Épic :** E06 — La fiche d'équipe complétée · **Ordre :** 2
**Dépend de :** 476, **434**
**Conception :** `docs/specs/matchs-d-une-equipe/README.md`, et
`docs/specs/fiche-equipe-onglets/README.md` pour le mécanisme d'onglets
**Remplace :** la carte `match-02-team-matches-tab`

## Objectif

`teams-team-detail.html:147` porte `<div class="tab">Matchs</div>` — un libellé
sans `hx-get`, sans handler, sans fragment. On clique, rien ne se passe.

L'onglet liste les matchs de la saison courante de l'équipe, à venir compris.

## Qui sert le fragment — et pourquoi ce n'est pas `match_report`

La carte `match-02` posait `match_report` comme émetteur, et demandait d'y créer
un « modèle de persistance des matchs ».

**Il n'y a rien à modéliser.** `competition_match_display_proj` porte déjà, par
match, les identifiants et les noms dénormalisés des deux équipes, leurs
rosters, leurs coachs, leurs logos, le score, les blessures et l'URL du rapport.
Et cette table appartient à **`competitions`**.

C'est donc `competitions` qui expose le fragment, et la fiche d'équipe qui le
compose — le patron déjà en place à la ligne 153 de la même fiche :

```rust
players_widget_url: app_routes.players.players_by_team_widget(space_id, &team.id.to_string()),
```

## Deux routes, et pourquoi

L'onglet appartient à `teams`, son contenu à `competitions`. **Pointer l'onglet
directement sur la route de `competitions` ne marche pas** : le `hx-push-url`
mettrait une URL `/competitions/…` dans la barre d'adresse pendant qu'on regarde
une fiche d'équipe, et un rechargement livrerait le fragment nu, sans la fiche
autour.

```
GET /app/{space_id}/teams/{team_id}/matchs                    ← teams, la coquille
GET /app/{space_id}/competitions/teams/{team_id}/matches      ← competitions, le fragment
```

La coquille est trois lignes, sur le patron exact du widget joueurs déjà en
place à la ligne 153 de la même fiche :

```html
<div id="team-matches" hx-get="{{ vm.matches_widget_url }}"
     hx-trigger="load" hx-target="this" hx-swap="outerHTML">
  <div class="loading-placeholder">Chargement des matchs…</div>
</div>
```

Le prix est **un aller-retour de plus au premier clic**. C'est celui de la
souveraineté des données, et il est déjà payé pour les joueurs.

`TEAM_MATCHES` est posée **par cette carte** et non par la 434 : une route montée
sans contenu répond « rien », et la règle est qu'un onglet ne devient cliquable
que lorsque son contenu existe.

## La route du fragment

`{team_id}` est **déjà scopé** : `TeamSpaceOwnership` le résout, et il consulte
les deux sources depuis les cartes 320-321. Rien à ajouter au middleware — et
surtout pas un second résolveur, qui serait une erreur de démarrage.

## La requête

```sql
-- sql/match_days/list_team_matches.sql
SELECT  … le SELECT de list_resultats.sql, inchangé …
FROM competition_match_display_proj
WHERE season_id = $1
  AND (home_team_id = $2 OR away_team_id = $2)
ORDER BY
    CASE match_status WHEN 'in_progress' THEN 0 WHEN 'upcoming' THEN 1 ELSE 2 END,
    CASE WHEN match_status = 'upcoming' THEN round_position END ASC,
    round_position DESC
```

**Le `SELECT` est celui de `list_resultats.sql` au caractère près** : c'est ce
qui garantit que le même `MatchResultatVm` se construit sans adaptation.

**Aucun filtre de statut** — les trois sont voulus.

### L'ordre n'est pas celui de la compétition

L'onglet compétition trie `round_position DESC` et exclut les matchs à venir.
Reprendre ce tri en les incluant mettrait **le match le plus lointain en tête**
et enterrerait le prochain au milieu.

D'où la chronologie centrée sur maintenant :

```
1. le match en cours de saisie
2. les matchs à venir, du plus proche au plus lointain
3. les matchs joués, du plus récent au plus ancien
```

Sur une fiche d'équipe, « mon prochain match » est ce qu'un coach vient
chercher. Le double `CASE` mérite son commentaire dans le fichier : deux tris
opposés dans une requête se lisent mal.

### Pas de pagination

L'onglet compétition pagine — une saison, c'est des centaines de matchs. **Une
équipe en joue dix à quinze.** Ni `scroll-sentinel`, ni curseur, ni `LIMIT`.

### La saison courante seulement

Une équipe **peut jouer plusieurs saisons** (`TeamEnrolled`). Sans le filtre,
« Journée 1 » reviendrait à chaque saison en groupes homonymes, et le curseur
`round_position` se répéterait. La décision évite les trois problèmes d'un coup.

## Le contrôle d'accès — `compute_authorization` convient verbatim

```rust
let enr   = team_info_port.find_team_enrollment(team_id).await?;    // ← neuf
let authz = compute_authorization(state, user, space_id,
                                  &CompetitionId::try_new(&enr.competition_id)?,
                                  &enr.season_id).await;            // ← existant
```

Elle rend exactement la règle voulue :

| Qui consulte la fiche de A | Lien vers le rapport |
|---|---|
| admin d'espace ou de la compétition | oui |
| le coach de A | oui |
| le coach de B, sur un match A–B | **oui — il a joué ce match** |
| le coach de B, sur un match A–C | non |

La troisième ligne vaut mieux qu'une règle écrite à la main : un coach garde
l'accès aux rapports de **ses** matchs où qu'il les regarde. La lui retirer ici
créerait une incohérence avec l'onglet Résultats, qui montre le même match.

### Le piège : la compétition ne se prend pas dans l'URL

Baker `competition_id` dans l'URL du widget serait naturel — c'est ce que la
règle 4 des widgets demande pour les paramètres contextuels. **Il ne faut pas.**

```
utilisateur admin de la compétition X
→ ouvre la fiche d'une équipe de la compétition Y
→ force ?competition_id=X
→ is_comp_admin = vrai
→ liens vers les rapports de Y
```

`space_scope` ne l'attrape pas : X et Y sont dans le même espace, les deux
résolvent. Le `competition_id` **se résout côté serveur depuis le `team_id`**,
seul identifiant du chemin.

```rust
// competitions/ports.rs — à côté de find_enrolled_teams et find_team_names
async fn find_team_enrollment(&self, team_id: &str)
    -> Result<Option<TeamEnrollmentDto>, String>;

pub struct TeamEnrollmentDto { pub competition_id: String, pub season_id: String }
```

Une équipe sans inscription — brouillon non soumis — rend une **liste vide et
non une erreur** : elle n'a joué aucun match, ce qui est vrai.

## La liste est plate

L'onglet compétition groupe par journée : « Journée 3 · 6 matchs », puis six
blocs. **Une équipe joue un match par journée** — quinze groupes d'un match,
chacun titré « 1 match ».

Le libellé passe donc *dans* le bloc, par le `round_label` de la carte 476.

## L'état vide

> Aucun match pour le moment.

Le même que l'onglet compétition, au mot près. Il couvre les deux cas : pas
encore de calendrier, et brouillon non inscrit.

## Tests

| Test | Ce qu'il prouve |
|---|---|
| `une_victoire_a_domicile_donne_la_pastille_v` | la dérivation |
| `une_victoire_a_l_exterieur_donne_aussi_la_pastille_v` | **le sens de `is_home`** |
| `un_score_egal_donne_la_pastille_n` | le nul |
| `un_match_a_venir_n_a_pas_de_pastille` | `None`, pas « N » |
| `une_equipe_sans_inscription_rend_une_liste_vide` | le brouillon |
| `les_matchs_d_une_autre_saison_ne_remontent_pas` | le filtre de saison *(intégration)* |
| `un_match_a_domicile_et_un_a_l_exterieur_remontent` | le `OR` *(intégration)* |
| `le_prochain_match_est_en_tete` | l'ordre *(intégration)* |
| `un_match_en_cours_passe_devant_les_a_venir` | l'ordre, seconde moitié *(intégration)* |

`une_victoire_a_l_exterieur_donne_aussi_la_pastille_v` est celui qui compte :
une inversion de `is_home` donne une pastille **fausse une fois sur deux**, ce
qui ressemble à un défaut de données et non de code.

## Ce que la carte ne fait pas

- **Aucun historique multi-saison.**
- **Aucun bilan V/N/D agrégé** — c'est la carte `match-01` de la même épic. Sa
  source est désormais connue : cette requête, comptée.
- **Aucun filtre ni tri choisis par l'utilisateur.**

## Checklist

- [ ] `sql/match_days/list_team_matches.sql`
- [ ] `find_team_enrollment` sur `ITeamInfoPort` + son adapter
- [ ] La route, le contrôleur, `team-matches-widget.html` incluant le composant
- [ ] `compute_authorization` **réutilisée**, aucun contrôle d'accès neuf
- [ ] Le `competition_id` résolu depuis le `team_id`, **jamais depuis l'URL**
- [ ] `TEAM_MATCHES` côté `teams`, la coquille `teams-matches-tab.html`
- [ ] L'onglet « Matchs » passe de `<div>` inerte à `<a>` htmx
- [ ] `matches_widget_url` via `AppRoutes`, jamais un import direct
- [ ] Les neuf tests
- [ ] `make lint && make test && make check-arch`
