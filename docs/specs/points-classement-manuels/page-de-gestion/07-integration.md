# Points de classement manuels · Phase 7 : effets de bord

**Phase 6** : `06-domaine.md`

## Persistance

### La table

```sql
-- migrations/<date>_ranking_manual_points.sql
CREATE TABLE ranking__manual_points (
    id          BIGSERIAL PRIMARY KEY,
    season_id   TEXT NOT NULL,
    team_id     TEXT NOT NULL,
    points      INTEGER NOT NULL,
    reason      TEXT,
    awarded_by  TEXT NOT NULL,
    awarded_at  TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX ON ranking__manual_points (season_id);
CREATE INDEX ON ranking__manual_points (season_id, team_id);
```

**Aucune migration de données** : la table naît vide, et rien d'existant n'est
touché.

Deux index parce que les deux lectures diffèrent : le classement agrège par
saison, la page de gestion liste par saison **et** groupe par équipe.

### Quatre méthodes sur `IRankingRepository`

```rust
/// Le total par équipe — pour le classement.
async fn find_manual_totals_for_season(&self, season_id: &str)
    -> Result<HashMap<String, i32>, RankingRepositoryError>;

/// Le détail — pour la page de gestion.
async fn list_manual_points(&self, season_id: &str)
    -> Result<Vec<ManualPointRow>, RankingRepositoryError>;

async fn insert_manual_points(&self, …) -> Result<(), RankingRepositoryError>;

/// Rend le nombre de lignes supprimées : zéro devient `NotFound`.
async fn delete_manual_points(&self, id: i64, season_id: &str)
    -> Result<u64, RankingRepositoryError>;
```

```sql
-- le total, agrégé par la base
SELECT team_id, SUM(points)::int AS total
FROM   ranking__manual_points WHERE season_id = $1 GROUP BY team_id

-- la suppression, saison comprise (phase 5)
DELETE FROM ranking__manual_points WHERE id = $1 AND season_id = $2
```

Le `SUM` est fait par Postgres : sommer en Rust demanderait de remonter chaque
ligne pour n'en garder qu'un nombre par équipe.

## Événements

**Aucun.** Le classement s'ordonne à chaque lecture (phase 3) : rien à propager,
rien à invalider, aucun autre BC ne connaît les points manuels.

C'est la troisième fonctionnalité d'affilée dont la phase 7 le dit — mais ici la
raison est structurelle et vaut d'être répétée : **il n'existe aucune table de
classement ordonné.** `build_ordered_standings` recalcule l'ordre à chaque
affichage, ce qui rend toute propagation inutile.

## Handlers

```
ranking/io/web/manual_points/
├── mod.rs
├── manual_points_page.rs      GET    …/manual-points
├── manual_points_form.rs      GET    …/manual-points/form
├── manual_points_list.rs      GET    …/manual-points/list
└── manual_points_actions.rs   POST   …/manual-points
                               DELETE …/manual-points/{point_id}
```

Cinq routes, à ajouter à `routes.rs` et au routeur — qui n'en compte que deux
aujourd'hui.

### Le contrôle d'accès, et ce qui le porte

| Route | Qui |
|---|---|
| les trois `GET` | **tout membre de l'espace** — les points sont publics (C4) |
| `POST`, `DELETE` | admin de compétition ou d'espace (A1) |

`space_scope` couvre `{season_id}`, dont `competitions` déclare le résolveur :
une saison d'un autre espace rend `404` **avant** le handler.

**`{point_id}` n'est résolu par personne**, et c'est le `AND season_id = $2` du
`DELETE` qui referme ce trou — par construction plutôt que par un contrôle à
écrire (phase 5).

`can_manage` est calculé une fois par rendu et passé au VM : le gabarit ne
décide pas, il rend ou non la colonne de suppression.

### Les sorties

| Cas | Réponse |
|---|---|
| POST réussi | le formulaire re-rendu vide + `HX-Trigger: manualPointsChanged` |
| DELETE réussi | `204` + `HX-Trigger: manualPointsChanged` |
| value object refusé | `422` + le formulaire, l'erreur nommant le champ |
| `TeamNotEnrolled` | `422` |
| `Forbidden` | `403` |
| `NotFound` (ligne absente) | `404` |

**Le POST rend le formulaire vidé de ses points et de son motif, mais
l'équipe reste choisie.** C'est le geste réel : un arbitre qui traite les
forfaits d'une journée enchaîne plusieurs attributions, souvent sur des équipes
différentes mais parfois sur la même. Tout réinitialiser lui ferait re-choisir à
chaque fois ; ne rien réinitialiser lui ferait attribuer deux fois le même
nombre par inadvertance.

**`HX-Trigger` sur les deux mutations**, et la liste seule l'écoute
(`hx-trigger="manualPointsChanged from:body"`). Le formulaire n'écoute rien
(phase 2).

## Le classement — ce qui change

### Deux widgets, une lecture de plus

`classement_widget.rs:53` et `detailed_standings_widget.rs:96` ajoutent
`find_manual_totals_for_season` à leur `tokio::join!` existant — **quatre
requêtes deviennent cinq, en parallèle**, sans allonger le temps de réponse.

```rust
let (rules, teams, lines, groups, manual) = tokio::join!(…);
```

Puis `build_ordered_standings(lines, &manual, &order)`.

### Deux colonnes de gabarit

| Gabarit | Où |
|---|---|
| `classement-widget.html` | entre `D` et `Pts` — « Man. » |
| `detailed-standings-widget.html` | entre `Bonus` et `Total` — « Manuel », et le `colspan` du groupe « Points » passe de 2 à 3 |

**Un point manuel non nul est un lien** vers la page de gestion ; le tiret d'un
zéro n'en est pas un.

### Le bouton d'accès

Dans les **deux** onglets de classement, et nulle part ailleurs. Il ne s'affiche
qu'aux administrateurs.

**La duplication est délibérée** et le commentaire du gabarit doit le dire : la
page de compétition compte six onglets, et au-dessus d'eux le bouton
s'afficherait sur Calendrier, Équipes et Statistiques.

## CSS

Une feuille neuve, `pages/ranking-manual-points.css`, portée par `.mp-page`, à
**inscrire dans `src/web/css_bundle.rs`** — l'axe 14 refuse toute feuille
absente du bundle.

Les colonnes s'ajoutent aux deux feuilles existantes, déjà au bundle
(lignes 128-129).

> ### La carte 448 se traite dans le même geste
>
> `widgets/ranking-detailed-standings-widget.css` met le zébrage en `--dark-7`
> (ligne 40) et le survol en `--dark-6` (ligne 44) — **deux valeurs séparées par
> un rapport de 1,0012**, donc invisibles l'une de l'autre. Le survol du
> classement ne se voit pas une ligne sur deux, aujourd'hui, en production.
>
> Ajouter une colonne à ce tableau sans corriger cela livrerait une nouveauté
> dans un écran déjà cassé. **La 448 passe avant, ou avec.**

## Tests E2E

`tests/e2e/test_manual_ranking_points.py`.

| Scénario | Ce qu'il prouve |
|---|---|
| `test_attribuer_des_points_a_une_equipe` | le chemin heureux |
| `test_le_classement_affiche_les_points_manuels` | la colonne, dans les deux vues |
| `test_le_classement_est_reordonne_sans_rechargement_du_serveur` | **le test qui compte** |
| `test_une_penalite_fait_descendre_l_equipe` | C3 |
| `test_supprimer_une_ligne_la_retire_du_classement` | le retour en arrière |
| `test_un_non_admin_voit_la_page_sans_les_actions` | A1 et C4 ensemble |
| `test_la_liste_se_recharge_apres_attribution` | `HX-Trigger` |

**`test_le_classement_est_reordonne_sans_rechargement_du_serveur`** vaut le prix
de la suite : il attribue assez de points pour changer l'ordre, recharge la page
de classement, et vérifie que les rangs ont suivi — **sans redémarrage**. C'est
lui qui prouve que la phase 3 avait raison, et qu'aucune propagation n'est due.

**Les trois suites existantes doivent rester vertes sans modification** :
`test_detailed_standings.py`, `test_ranking_bonus.py`,
`test_ranking_tiebreak.py`. Elles n'attribuent aucun point manuel, donc elles
mesurent la non-régression du classement ordinaire.

## Ce que la phase ne prévoit pas

- **Aucun événement, aucun listener, aucun rejeu.**
- **Aucune migration de données.**
- **Aucun changement à l'écriture de `ranking_lines`** ni aux deux listeners
  existants du BC.

## Règles métier

**Aucune à préciser.** Les douze de la phase 6 couvrent la fonctionnalité.
