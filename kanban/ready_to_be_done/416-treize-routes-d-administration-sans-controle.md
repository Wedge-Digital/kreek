# Treize routes d'administration sans contrôle d'accès

**Priorité : haute** — un membre d'espace non administrateur peut détruire le
calendrier d'une compétition qu'il ne gère pas
**Périmètre : la couche web du BC `competitions`**
**Trouvée par :** la phase 7 de l'onglet Paramètres, en instruisant les
signatures de handlers
**Dépend de :** rien

## Le constat

`groups_actions.rs` et `schedule_actions.rs` exposent treize routes de mutation.
**Aucune n'accepte `AuthSession`. Aucune n'appelle `require_admin_access`.**

| Fichier | Routes |
|---|---|
| `groups_actions.rs` | `post_random_draw`, `post_reset_groups`, `post_assign_team` |
| `schedule_actions.rs` | `post_generate_all`, `post_clear_all`, `post_add_round`, `post_add_rest`, `put_update_round`, `delete_round`, `post_generate_round_pairings`, `post_clear_round_pairings`, `post_add_match`, `delete_match` |

Ce qu'un utilisateur simplement connecté peut donc faire : régénérer tout le
calendrier, vider une journée, supprimer une journée, supprimer un match,
réinitialiser la répartition en poules, réaffecter une équipe.

La fonction qui manque **existe** — `admin_page.rs:57` — et son commentaire
énonce exactement la règle que ces treize routes enfreignent :

> À appeler sur **chaque** route admin (page complète ET fragment htmx), pas
> seulement sur le chargement de page complet.

Elle n'est appelée que par le rendu des onglets.

## Le second défaut, plus grave

Plusieurs handlers prennent leur cible **dans le corps de la requête** et
ignorent les identifiants de chemin :

```rust
pub async fn delete_match(
    Path((_space_id, _competition_id, _season_id)): Path<(String, String, String)>,
    State(state): State<AppState>,
    axum::Json(body): axum::Json<DeleteMatchBody>,   // ← la vraie cible
) -> Response
```

Même forme pour `post_assign_team` (`team_id`, `group_id` dans le corps),
`delete_round` et `post_clear_round_pairings`.

Or le middleware `space_scope` ne résout que les paramètres **de chemin** : ses
résolveurs déclarent `competition_id`, `season_id`, `team_id`, `player_id`,
`match_report_id`, `article_id`. Une cible passée dans le corps n'est vue par
personne.

**Conséquence** : l'identifiant visé n'a pas à appartenir à l'espace de l'URL.
Un coach qui participe à une compétition — donc qui connaît les identifiants de
ses propres appariements — peut les faire supprimer en postant sur une URL
d'administration qu'il a le droit d'atteindre par ailleurs.

## Ce que la carte fait

1. **`AuthSession` + `require_admin_access` en première ligne des treize.**
2. **Aucune cible hors du chemin.** Les identifiants du corps passent dans
   l'URL, ou le handler vérifie explicitement que la cible appartient à la
   saison du chemin. La première voie est préférable : elle rend le contrôle
   structurel plutôt que répété.
3. **Un test par route** vérifiant le `403` — c'est le seul moyen que la
   régression ne repasse pas inaperçue, puisque rien dans le compilateur ne
   signale un handler qui ne contrôle rien.

## Ce qui rend la correction sûre

`require_admin_access` rend déjà `Result<CompetitionBaseInfo, Response>` : les
handlers qui ont besoin des informations de compétition les obtiennent du même
appel, sans requête supplémentaire.

Les treize handlers rendent tous `Response` ou `impl IntoResponse` — le `403`
s'y insère sans changer leur signature de retour.

## Ce que la carte ne fait pas

**Elle ne touche pas aux GET.** Les fragments de lecture de l'administration
sont dans le même cas et méritent le même traitement, mais une lecture non
autorisée et une destruction non autorisée ne sont pas la même urgence. À
traiter ensuite, pas dans la même carte.

## Vérification préalable

Avant de corriger, mesurer si l'accès non autorisé a servi :

```sql
-- les journées et appariements supprimés n'ont pas de trace en base ;
-- le journal applicatif, lui, porte les use cases appelés et leur `rid`.
```

Le projet journalise chaque use case (`#[tracing::instrument]`, épic E11). Un
`grep delete_pairing_use_case` sur les journaux de production dira si des
suppressions ont eu lieu, et par quel `rid` les retrouver.
