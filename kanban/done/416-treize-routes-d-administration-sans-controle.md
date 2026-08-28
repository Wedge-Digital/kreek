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

## Passer les identifiants dans l'URL n'aurait rien contrôlé

La carte prescrivait de faire remonter les cibles du corps vers le chemin, et
disait : « **La première voie est préférable : elle rend le contrôle structurel
plutôt que répété.** »

C'est faux pour ces identifiants-là. La docstring de `space_scope` le dit
elle-même, deux lignes après en avoir tiré la conclusion inverse :

> Les paramètres sans résolveur (`round_id`, `pairing_id`, `action_id`…)
> passent : ils sont toujours accompagnés d'un parent qui, lui, est contrôlé.

La prémisse est vraie, la conclusion ne l'est pas. Le parent — `season_id` — est
bien contrôlé pour l'espace ; **rien ne rattache l'enfant au parent**. Un
`round_id` reste aussi libre dans le chemin que dans le corps. Le déplacer
n'aurait changé que la forme, en touchant cinq appels JS dans deux templates et
en mêlant un changement cosmétique à un correctif de sécurité qu'on voudra
pouvoir relire seul.

**Le contrôle est donc explicite**, dans `io/web/admin/admin_scope.rs` : la
journée, l'appariement, le groupe et l'équipe visés appartiennent-ils à la saison
du chemin ? Une lecture chacun, aucun nouveau port.

`404` et non `403` sur une cible étrangère : un `403` confirmerait son existence
à qui se contente d'essayer des identifiants.

## Deux failles de plus, que la carte n'avait pas vues

**`require_admin_access` ne recevait pas `season_id`.** Il vérifiait l'espace et
la compétition. Le droit étant accordé **par compétition**, l'administrateur de
la compétition A pouvait poser son propre `competition_id` et le `season_id` de
la compétition B du même espace : la garde passait, puis le handler agissait sur
B. `space_scope` ne le rattrape pas — il vérifie que la saison appartient à
l'**espace**, jamais à la compétition.

Le contrôle est posé **dans `require_admin_access`** plutôt que dans chaque
handler : la fonction devient le seul endroit qui réponde « ce chemin est-il
cohérent et m'est-il permis ? », et ses sept appelants en bénéficient sans y
penser.

**`put_update_round` et `delete_round` prennent `round_id` dans le chemin** — et
la carte les rangeait donc parmi les routes sans problème de cible. Elles
chargeaient pourtant la journée par son seul identifiant, en ignorant
`_season_id` : exactement le même trou que les cibles du corps, sans en être.

## Les tests

Six scénarios dans `tests/e2e/test_competition_admin_acces.py`, sur **deux**
compétitions du même espace — une cible inventée ne prouverait rien, elle rendrait
`404` pour la seule raison qu'elle n'existe pas.

Un test paramétré couvre les treize routes plutôt qu'un test chacune : la liste
**est** l'assertion, et un décompte final interdit d'en retirer une sans s'en
apercevoir. Le corps envoyé est celui du vrai client — un corps absent ferait
échouer l'extracteur *avant* le contrôle d'accès, et le test vérifierait un rejet
de format au lieu d'un refus de droit.

**Cinq mutations, cinq tests rouges, chacun le sien** — la garde de
`delete_match`, l'appartenance de `post_add_match`, celle de l'appariement, celle
du groupe, et la cohérence saison↔compétition. La contre-épreuve reste verte.
`post_add_match` est visée délibérément : c'est la **dernière** route de la boucle
du test de portée, donc aussi la preuve que la boucle va jusqu'au bout.

## Un piège rencontré : un test sauté n'est pas une couverture

Le scénario de portée des poules se **sautait** — `competition_groups` était vide.
Les poules sont configurées par le magicien mais ne se matérialisent qu'au premier
affichage de leur widget (`ensure_groups_from_structure`). Le test ouvre désormais
ce widget avant de viser un groupe, et affirme qu'il en trouve un.

## Vérification préalable

Avant de corriger, mesurer si l'accès non autorisé a servi :

```sql
-- les journées et appariements supprimés n'ont pas de trace en base ;
-- le journal applicatif, lui, porte les use cases appelés et leur `rid`.
```

Le projet journalise chaque use case (`#[tracing::instrument]`, épic E11). Un
`grep delete_pairing_use_case` sur les journaux de production dira si des
suppressions ont eu lieu, et par quel `rid` les retrouver.
