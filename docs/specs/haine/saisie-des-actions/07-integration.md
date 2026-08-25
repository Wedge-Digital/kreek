# Saisie des actions — gain de la Haine · Phase 7 : effets de bord

**Entrée** : `06-domaine.md` validé. Conception, pas implémentation.

## 1. Persistance — aucune migration

La projection des actions stocke l'action **sérialisée en JSONB** :

```sql
INSERT INTO match_report_actions
  (action_id, match_report_id, team_side, turn_number, player_id,
   player_type, action_json, player_display_name, player_position)
```

`action_json` absorbe `Blesse { injury, hatred }` sans changer de schéma. **Rien
à migrer, aucune colonne à ajouter**, ni pour la projection, ni pour l'event
store — qui sérialise déjà les événements de la même façon.

**En contrepartie, `#[serde(default)]` est obligatoire** sur le nouveau champ :

```rust
Blesse {
    injury: InjuryType,
    #[serde(default)]
    hatred: Option<HatredKeyword>,
},
```

Sans lui, **toutes les actions déjà écrites deviendraient illisibles** — leur
JSON ne porte pas le champ, et le rejeu de l'event store échouerait sur chaque
blessure historique. C'est la convention du projet, pas une précaution
inventée : `events.rs` porte déjà six `#[serde(default)]` posés pour la même
raison, et `MatchAction::player_position` en est l'exemple le plus proche.

Aucune méthode de repository nouvelle.

## 2. Événements — R9 est déjà tenue

`build_player_impact_events` (`app_event_publisher.rs:318`) filtre **déjà** les
joueurs non permanents :

```rust
let ActionPlayer::Regular(player_id) = &a.player else {
    return None; // BR1 — stars/mercenaires/journaliers exclus
};
```

**R9 ne coûte donc rien** : la Haine d'un journalier est enregistrée dans
l'action, visible au récapitulatif, et ne traverse pas vers `players` — sans une
ligne de code, par un filtre qui existait avant elle.

Deux changements seulement :

```rust
// app_event_publisher.rs — la conversion
MatchActionType::Blesse { injury, hatred } => PlayerMatchImpactAppEvent::PlayerInjured {
    context,
    injury_type: map_injury_type_payload(injury),
    hatred: hatred.as_ref().map(|k| k.to_string()),
},
```

```rust
// player_match_impact_listener.rs — l'application, à la conclusion du match
// Un PlayerInjured porteur d'un mot-clef produit, en plus de l'impact de
// blessure, un gain de trait : player.record_hatred(context, keyword, nom).
```

Le nom affiché (« Haine : Nain ») est **résolu au catalogue par le listener**,
pas transporté par l'app event : un libellé change avec le corpus, un uid non.
`players` a déjà `ISkillCatalogPort` pour ça.

**La dépublication est gratuite elle aussi** : `TeamMatchImpactReverted` défait
l'impact du match sur tout l'effectif, la Haine comprise, sans traitement
particulier. C'était l'argument pour ne pas créer un second chemin.

## 3. Handlers

```rust
// GET — le panneau, deux listes de plus
pub async fn get_action_panel_step3(
    Path((space_id, mr_id)): Path<(String, String)>,
    Query(params): Query<ActionPanelParams>,
    State(state): State<AppState>,
) -> Response
```

Il cesse d'être inerte — il commençait par `let _ = state;`. Il appelle
`hate_keywords_service::choices(...)`, transforme en VMs, et rend le template.
Aucune logique : le partage des listes et le tri appartiennent au service.

```rust
// POST — inchangé dans sa forme
pub async fn post_action_step3(
    auth_session: AuthSession,
    Path((space_id, mr_id)): Path<(String, String)>,
    State(state): State<AppState>,
    Form(form): Form<RecordActionForm>,
) -> Response
```

Il gagne **un seul refus** : `hate_gained = Some(true)` sans `hate_keyword` → 422
avant construction de la commande. Les deux autres refus viennent du use case et
du domaine, et se traduisent aussi en 422 :

| Erreur | Statut | Journal |
|---|---|---|
| `UnknownKeyword(uid)` | 422 | `warn`, avec l'uid |
| `Domain(HatredNotAllowedForInjury)` | 422 | `warn`, le type de blessure étant déjà dans le span |

## 4. Templates

| Template | Change |
|---|---|
| `action-panel-widget.html` | la section Haine : question, filtre, deux groupes, repli. Reprise de la maquette |
| `action-log-widget.html` | la ligne de blessure affiche « + Haine : Nain » |

Le widget garde son `hx-disinherit="*"` et son `x-data` : la Haine ajoute quatre
propriétés (`hateGained`, `hateKeyword`, filtre, repli) à un composant qui en
porte déjà quatre. Pas de `<script>` nu, pas d'`id` global — conventions 6 et 7
des widgets.

Les deux autres écrans — fiche d'équipe, fiche joueur — sont **d'autres pages**
du même workflow, avec leurs propres phases 2 à 7.

## 5. Tests E2E prévus

| Scénario | Ce qu'il vérifie |
|---|---|
| Blessure Amoché → section visible, réponse Non → action enregistrée sans Haine | R1, R2 |
| Blessure Séquelle → Oui → mot-clef → journal affiche la Haine | chemin nominal |
| Commotion → **la section n'apparaît pas** | R1 côté front |
| Oui sans mot-clef → **le bouton de confirmation reste masqué** | R3 côté front |
| Filtre « yéti » → le repli s'ouvre seul et le mot apparaît | ergonomie |
| Mot-clef choisi puis filtre ne le contenant pas → il reste visible | ergonomie |
| Deux Haines identiques sur le même joueur → **acceptées** | R7 |
| Journalier blessé avec Haine → action enregistrée, joueur inchangé après publication | R9 |

**Le troisième et le quatrième sont les plus utiles** : ils vérifient qu'une
chose **n'apparaît pas**, ce qu'aucun test unitaire ne peut voir. Le `CLAUDE.md`
le dit du widget coach-search et des pickers de tiers — ces défauts-là ne se
révèlent qu'en navigateur.

Le dernier demande un rapport publié : c'est le seul scénario long, et il vérifie
la seule règle dont l'effet est une **absence** d'écriture ailleurs.
