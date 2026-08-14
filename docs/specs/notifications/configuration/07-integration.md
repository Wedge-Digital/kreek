# Phase 7 — Effets de bord : l'écran de réglage des notifications

**Entrée** : `06-domaine.md`, validée. Cette phase est de la conception ; rien
n'est codé ici.

## Le piège de cette phase : une case décochée n'est pas envoyée

En mode auto-save, le widget POSTe par `hx-post` — donc en encodage de
formulaire. **Une case à cocher non cochée n'apparaît pas dans le corps de la
requête.** Un DTO qui exige ses quatre champs échouerait à désérialiser dès la
première case décochée.

Conséquences, à ne pas rater à l'implémentation :

```rust
#[derive(Debug, Deserialize)]
pub struct NotificationSettingsPayload {
    #[serde(default)] pub registration_open: bool,
    #[serde(default)] pub round_eve: bool,
    #[serde(default)] pub round_closing: bool,
    #[serde(default)] pub registration_deadline: bool,
}
```

et l'extracteur est **`Form`, pas `Json`** — la phase 4 écrivait `Json` par
symétrie avec l'étape 4 du magicien, qui, elle, envoie du JSON par `fetch`.

Le symptôme d'une erreur ici serait trompeur : on pourrait activer une
notification et jamais la désactiver, ce qui ressemble à un défaut de
persistance alors que c'est le corps de la requête qui est incomplet.

## Persistance

### La migration

```sql
-- migrations/<ts>_competition_season_notifications.sql

ALTER TABLE competition_seasons ADD COLUMN IF NOT EXISTS notifications JSONB;

-- R8 : les saisons existantes démarrent éteintes.
-- Ce remplissage n'est pas une commodité — il est ce qui donne à NULL un sens
-- unique. Sans lui, une colonne absente voudrait dire à la fois « ancienne
-- saison, donc éteint » et « saison neuve, donc allumé », et rien dans la ligne
-- ne permettrait de trancher : le statut n'y suffit pas,
-- `invitations_configured` désignant aussi bien une saison abandonnée en cours
-- de magicien qu'une saison d'avant cette migration.
UPDATE competition_seasons
SET    notifications = '{"registration_open":false,"round_eve":false,
                         "round_closing":false,"registration_deadline":false}'::jsonb
WHERE  notifications IS NULL;
```

Après cette migration, `NULL` signifie « créée après », et le défaut serde —
les quatre à `true` — s'applique sans ambiguïté.

### Les deux requêtes

`select_notifications.sql`, et les deux `UPDATE` décrits en phase 5 :
`update_invitations.sql` gagne la colonne et **garde** son `status` — c'est bien
le magicien ; `update_notifications.sql` est nouveau et **n'écrit pas** de
statut, faute de quoi une compétition vivante retomberait dans le magicien.

### Le port

`ISeasonRepository` gagne `find_notifications` et `save_notifications`, et
`save_invitations` change de signature (un seul appelant). Détail en phase 5.

## Événements

**Aucun.** Justifié en phase 5 : `envoi/` lit la colonne au moment du cron, ce
qui est une consultation d'état au présent, donc un port et non un app event.
Un `NotificationSettingsChanged` n'aurait aucun abonné.

## Handlers

```rust
// widgets/notification_settings_widget.rs

pub async fn get_notification_settings_widget(
    Path((space_id, competition_id, season_id)): Path<(String, String, String)>,
    Query(q): Query<WidgetModeQuery>,          // mode = deferred | autosave
    State(state): State<AppState>,
) -> Result<impl IntoResponse, AppError>

pub async fn post_notification_settings(
    Path((space_id, competition_id, season_id)): Path<(String, String, String)>,
    State(state): State<AppState>,
    Form(payload): Form<NotificationSettingsPayload>,
) -> Result<impl IntoResponse, AppError>       // 204 No Content
```

Le GET charge trois choses — les réglages, la structure, les invitations —,
appelle `applicability()`, et bâtit le VM par `from_domain()`. Il ne calcule
rien lui-même.

Le POST bâtit les quatre value objects, la commande, appelle le use case, rend
`204`. Pas de fragment en retour : re-rendre le widget à chaque clic ferait
clignoter les cases et perdrait le focus clavier, pour réafficher ce qui est
déjà à l'écran.

**Un mode inconnu dans la query** vaut `400` et non un repli silencieux sur
`deferred` : un mode mal orthographié rendrait un widget muet, sans rien pour le
signaler.

**Le cloisonnement par espace n'est pas l'affaire de ces handlers** —
`SeasonSpaceOwnership` l'intercepte avant eux (phase 3).

**CSRF** : le middleware rejette les POST sans `HX-Request: true`. `hx-post` le
pose ; un `fetch` manuel devrait le poser à la main, comme le fait déjà
`submitInvitations()`.

### Le handler modifié de l'étape 4

`post_competition_invitations` reçoit désormais un corps portant le sous-objet
`notifications`, et transmet les deux à la commande. `notify_by_email` disparaît
du corps.

## Templates

| Fichier | Nature |
|---|---|
| `templates/widgets/notification-settings-widget.html` | **créé** — racine en `hx-disinherit="*"`, Alpine avec `init()`/`destroy()`, `<link>` vers son CSS |
| `assets/static/css/widgets/notification-settings.css` | **créé** — CSS embarqué, aucune dépendance au layout de l'hôte |
| `new-competition-phase-4.html` | **modifié** — la case `notify_by_email` cède la place au conteneur du widget ; la section 4 émet `registrationDeadlineChanged` à la frappe |
| `new-competition-phase-3.html` | **modifié** — retrait du bloc « Notifications e-mail », de `setMailNotif`, et des trois références à `state.useMailNotification` |
| `admin/summary.html` | **modifié** — conteneur du widget en mode auto-save |

Le VM consommé est `NotificationSettingsVm` (phase 4). Le template n'accède ni à
`CompetitionNotifications`, ni à `NotificationApplicability` — un template ne lit
pas un enum de domaine.

### Le retrait des deux interrupteurs morts

Retirer un champ d'une struct serde ne casse pas la lecture des blobs existants :
les clés inconnues sont ignorées. Aucune réécriture des ~399 blobs
`invitations` et `structure` n'est donc nécessaire, et il vaut mieux s'en
abstenir — le gain serait cosmétique, le risque réel.

`assets/league_structure.json` porte `"use_mail_notification": true` et doit être
nettoyé au même moment. La clé serait ignorée, mais un fichier de référence qui
mentionne un réglage disparu induit en erreur le prochain lecteur.

**Ordonnancement à surveiller** : `new-competition-phase-3.html` est aussi le
terrain de l'autre spec, celle du retrait des play-offs. Deux chantiers sur le
même template se gênent ; celui qui passe en second reprend le fichier de
l'autre.

## Tests E2E prévus

Le pytest/Playwright est le seul à voir ce qu'Alpine et HTMX produisent
réellement — la couverture unitaire de la phase 6 ne dit rien du rendu.

| Scénario | Ce qu'il garde |
|---|---|
| Étape 4 d'une saison neuve : les quatre cases présentes et cochées | R8, côté neuf |
| Décocher, continuer, revenir : l'état est persisté | la voie différée de bout en bout |
| Compétition sans calendrier : les deux lignes de journée grisées, avec leur motif | R5 et les motifs serveur |
| Effacer la date limite : la quatrième ligne grise **sans rechargement** | le grisage vivant — invisible à tout test unitaire |
| Cocher la date limite puis effacer la date : la case **reste cochée** et grisée | R6 |
| Admin : basculer un réglage sur une compétition démarrée, recharger, il a tenu | la voie auto-save |
| Admin : après cette bascule, la carte de la compétition mène **toujours au détail** | la non-régression du statut trouvée en phase 3 |

Le dernier est le plus important et le moins évident : il ne teste pas la
fonctionnalité mais le défaut qu'elle a failli introduire. Sans lui, un futur
`update_notifications.sql` qui reprendrait par mégarde la ligne `status` de son
voisin passerait tous les autres tests.

**`tests/impact-map.toml` doit être mis à jour dans le même commit** que ces
tests — une entrée manquante fait échouer `make check-arch` (axe 8), et un test
sans entrée est traité comme `"all"` puis signalé.

## Ce que cette phase laisse à la suivante

Le découpage en cartes. Trois blocs se dessinent — la migration et le port ; le
widget et ses deux hôtes ; le retrait des deux interrupteurs morts — mais c'est
la phase 8 qui tranche leur granularité et leur ordre.
