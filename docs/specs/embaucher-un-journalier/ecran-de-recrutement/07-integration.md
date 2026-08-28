# Écran de recrutement · Phase 7 : effets de bord

**Phase 6** : `06-domaine.md`

## Persistance — aucune table, quatre requêtes

### Une migration d'une ligne

```sql
-- migrations/<date>_players_membership_journeyman.sql
-- Rien à migrer : la variante s'ajoute, les 49 262 lignes `Active` ne bougent
-- pas. Cette migration n'existe que si une contrainte CHECK borne la colonne.
```

**À vérifier avant d'écrire** : si `players_proj.membership` porte un `CHECK`,
il doit accepter `'Journeyman'`. Sinon, **aucune migration n'est nécessaire** —
la colonne est un `TEXT`.

### Les quatre requêtes, et le changement silencieux

C'est le vrai travail de cette fonctionnalité (phase 3) :

| Fichier | Ligne | Devient |
|---|---|---|
| `players/io/repository/projection_repository.rs` | 29 | `membership <> 'Dismissed'` |
| idem | 130 | idem |
| idem | 148 | idem |
| `infrastructure/teams/squad_adapter.rs` | 47 | idem |

**Aucun compilateur ne le signalera** : ce sont des chaînes SQL. Une requête
oubliée donne un journalier invisible là où il devrait compter, **sans erreur** —
et le défaut se manifestera au rapport suivant, par un nombre de journaliers
faux.

**Le contrôle qui referme ça** :

```bash
grep -rn "membership = 'Active'" src/   # doit ne rien rendre
```

Une ligne dans la checklist vaut mieux qu'un espoir.

### Deux colonnes de plus dans la lecture d'effectif

`squad_adapter.rs:44` gagne `membership`, `acquired_skills` et les cinq deltas.
Mesuré : 298 joueurs sur 49 000 ont une compétence acquise, 47 une
caractéristique améliorée. Le coût est négligeable, et la requête sert déjà tout
l'écran.

## Événements — trois neufs, dont un qui manque au projet

```
match_report                      teams                        players
  init_temp_players                 ← JourneymenFielded
  │ JourneymenFielded               │ PlayerRecruited      →     crée en Journeyman
  ▼                                 │
                                  panier → validation
                                    │ JourneymanRecruited  →     bascule en Active
                                    │ RecruitmentPhaseValidated ←── N'EXISTE PAS
                                    ▼                            en app event
                                                          →     perd les restants
```

### Celui qui manque : `RecruitmentPhaseValidated`

Le **domain event** existe (`team.rs:159`) et fait passer l'équipe en
`Dismissals`. Mais `TeamsAppEvent` n'en compte que **deux** — `PlayerRecruited`
et `PlayerDismissed` — et aucun ne parle de phase.

Or la décision 13 dit : *« `players` fait le ménage lui-même, en écoutant la
sortie de la phase `Recruitment` »*. **Il lui faut donc un app event à écouter**,
qui n'existe pas.

```rust
TeamsAppEvent::RecruitmentPhaseValidated { event_id, team_id, space_id }
```

Le publisher de `teams` gagne un bras dans son `to_app_event()`. C'est un
événement **utile au-delà de cette fonctionnalité** : tout BC qui voudra réagir
à la fin d'une phase de recrutement l'aura.

### Les quatre listeners de `players`

| Écoute | Fait | Neuf ? |
|---|---|---|
| `JourneymenFielded` | crée les joueurs, `membership: Journeyman`, maillot par `premier_libre` | **oui** |
| `JourneymanRecruited` | bascule en `Active` — **sauf si `Dismissed`** | **oui** |
| `RecruitmentPhaseValidated` | passe les `Journeyman` restants en `Dismissed` | **oui** |
| `MatchReportCancelled` | **supprime** les journaliers de ce rapport | **oui** |

Le dernier suppose que `players` sache **lesquels** appartiennent à ce rapport.
Deux voies : l'événement porte les `player_id`, ou `players` retrouve les
`Journeyman` de l'équipe. **La première** — l'événement les porte — parce que la
seconde supprimerait aussi les journaliers d'un rapport antérieur non encore
traité, cas rare mais destructeur.

### Le garde-fou, et pourquoi il journalise

```rust
if player.membership == RosterMembership::Dismissed {
    tracing::warn!(player_id = %id, team_id = %team,
        "journalier recruté après avoir été perdu — teams a débité, le joueur reste sorti");
    return;
}
```

**Le débit a déjà eu lieu** quand ce listener s'exécute. Il ne peut pas
l'empêcher — mais cette ligne est ce qui permettra de rembourser à la main, et
c'est pour ça qu'elle porte les deux identifiants.

**Cible `kreek::`, sinon la ligne n'existe pas** : le filtre est
`kreek=<niveau>,sqlx=warn`, et une cible hors de ce préfixe n'est activée par
aucune directive. Un `tracing::warn!` depuis un module du projet en relève par
construction — mais il faut le vérifier plutôt que le supposer.

## Handlers

Une route de plus, dans le routeur de `teams` :

```rust
POST …/recruitment/journeyman/{player_id}   → post_add_journeyman
```

```rust
pub async fn post_add_journeyman(
    auth_session: AuthSession,
    Path((space_id, team_id, player_id)): Path<(String, String, String)>,
    State(state): State<AppState>,
) -> Response;
```

**Aucun corps** (phase 4). `space_scope` couvre `{team_id}`, dont `teams`
déclare le résolveur.

**`{player_id}` n'est pas résolu par le middleware** — mais il n'a pas besoin de
l'être : le use case ne le trouve que dans la liste des recrutables **de cette
équipe**, et un identifiant étranger donne `JourneymanNoLongerAvailable`. La
portée est tenue par la donnée, pas par un contrôle ajouté.

| Cas | Réponse |
|---|---|
| ajouté | `HX-Trigger: basketChanged` |
| `JourneymanNoLongerAvailable` | `422` + le catalogue re-rendu |
| `JourneymanAlreadyInBasket` | `422` |
| `SquadFull`, `InsufficientTreasury` | `422` |
| conflit de version | le conflit existant du panier |

**`basketChanged` fait tout le reste** : le catalogue et le panier se rechargent
tous deux, donc le journalier quitte la liste et apparaît dans le panier sans un
mécanisme de plus.

## Templates et CSS

`teams-recruitment-catalog.html` gagne le panneau, **au-dessus** de « Recruter
un joueur », rendu sous condition :

```
{% if !vm.journeymen.is_empty() %} … {% endif %}
```

**Les styles vont dans la feuille existante** — `widgets/rec-page.css`, déjà au
bundle. Aucune feuille neuve, donc rien à inscrire dans `css_bundle.rs`.

## `journeymen_value` — le commentaire qui manque

```rust
// team_value.rs:95
let missing = MATCH_SQUAD_SIZE.saturating_sub(available_count(players));
missing * journeyman_price.0
```

Dès que les journaliers sont de vrais joueurs, `available_count` les compte,
`missing` tombe à zéro, **la fonction rend zéro** — et le résultat reste juste
puisque `players_value` les compte.

**Le commentaire à ajouter est la livraison la plus importante de cette
section** : sans lui, quelqu'un croira la fonction morte et la supprimera,
cassant la valeur d'équipe de **toutes les équipes hors match** — pour
lesquelles la déduction est la seule source.

## Tests E2E

`tests/e2e/test_journeyman_recruitment.py`, et `test_recruitment_phase.py` reste
inchangé.

| Scénario | Ce qu'il prouve |
|---|---|
| `test_le_panneau_est_absent_sans_journalier` | le cas le plus fréquent |
| `test_un_journalier_apparait_apres_un_match` | la chaîne `match_report → teams → players` |
| **`test_le_journalier_recrute_reste_dans_l_effectif`** | **le test qui compte** |
| `test_le_journalier_non_recrute_disparait` | la décision 13, bout en bout |
| `test_le_prix_se_decompose_avec_une_amelioration` | « 65 + 20 » à l'écran |
| `test_le_meme_journalier_ne_s_ajoute_pas_deux_fois` | R5 |
| `test_seize_dont_journaliers_autorise_le_recrutement` | R4 — le cas qui donne son sens à la règle |

**`test_le_journalier_recrute_reste_dans_l_effectif`** traverse tout : la
création à l'ouverture du rapport, le match, la publication, le recrutement, la
sortie de phase — et vérifie qu'il est **toujours là** quand les autres sont
partis. C'est le seul qui prouve que l'ordre du lot d'événements (phase 5) tient.

**`test_recruitment_phase.py` doit rester vert sans une modification** : ses huit
cas ne concernent aucun journalier, donc ils mesurent la non-régression du
recrutement ordinaire.

## Ce que la phase ne prévoit pas

- **Aucune table neuve, aucune feuille CSS neuve.**
- **Aucun changement au déroulé du match** : le rapport garde ses `TempPlayer`.
- **Aucune reprise de l'existant** : les rapports en cours au moment de la
  livraison n'auront pas de journaliers dans `players`. Leur phase de
  recrutement n'en proposera aucun — comportement dégradé mais correct.

## Règles métier

**Aucune à préciser.** Cette phase révèle en revanche un manque : `TeamsAppEvent`
ne publiait aucun changement de phase, et la décision 13 en avait besoin.
