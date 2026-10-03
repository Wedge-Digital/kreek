# Les routes d'administration sans garde

**Priorité : haute — faille d'autorisation**
**Épic :** aucune — série 570 à 573
**Dépend de :** 570 et 572 (`est_admin` disponible dans `teams` et `competitions`)

## L'objectif

Toute route qui modifie ce qu'un commissaire administre vérifie le droit côté
serveur. Masquer un bouton n'est pas un contrôle d'accès.

## Ce qui l'a fait naître

L'inventaire de la carte 570 (2026-10-03) a trouvé des routes d'administration
**sans aucune vérification de droit** : leurs boutons ne s'affichent qu'aux
admins, mais une requête forgée par n'importe quel utilisateur connecté passe.
Seul `space_scope` vérifie la cohérence ressource/espace, pas le droit.

| Route | Fichier | Ce que n'importe quel connecté peut faire |
|---|---|---|
| approuver, rejeter, renvoyer une inscription, tout approuver | `teams/io/web/widgets/enrollment_actions.rs:23`, `:43`, `:67`, `:104` | décider des inscriptions d'une compétition |
| `post_update_competition` | `competitions/io/web/new_competition.rs:332`, `use_cases/update_draft_competition.rs:40-54` | **réécrire la liste des admins** de toute compétition de l'espace — se déclarer admin |
| `post_notification_settings` et son GET | `competitions/io/web/widgets/notification_settings_widget.rs:249`, `:203` | changer les notifications d'une compétition |
| GET des widgets de poules et de calendrier | `groups_widgets.rs:48`, `:141`, `schedule_widgets.rs:72`, `:270` | lire l'administration d'une compétition |
| `post_coach_creation_widget` | `auth/io/web/coach_creation_widget.rs:88` | créer un compte coach — alors que le panneau qui l'affiche est gardé (`spaces/.../create_coach_panel.rs:63`) |

Le commentaire de `garde_action_equipe.rs:23-25` affirme que les actions
d'inscription relèvent de « la règle du commissaire » : c'est faux depuis leur
création.

## Le changement

- **Inscriptions** (`teams`) : `est_admin` sur l'espace et la compétition de
  l'inscription ; 403 sinon.
- **Assistant de compétition** (`competitions`) : `post_update_competition`
  exige `est_admin` sur la compétition visée **et** un statut brouillon — un
  assistant de création ne modifie pas une compétition publiée.
- **Notifications et widgets d'administration** (`competitions`) : la garde de
  `require_admin_access`.
- **Création de compte** (`auth`, extractible) : `auth` ne peut dépendre ni de
  `shared_kernel::bloodbowl` ni de `spaces`. La garde passe par l'hôte — le
  contexte d'`auth` reçoit une fonction de droit injectée, sur le modèle de
  `ISpacesHostLayout`. **À concevoir avant de coder** : c'est le seul point de
  la carte qui touche au statut extractible.
- Le commentaire de `garde_action_equipe.rs` est corrigé.

## État — 2026-10-03

**Fait, et passé par la suite e2e complète** :
- inscriptions : approuver, rejeter, renvoyer exigent `exiger_commissaire` ;
  « tout approuver » vérifie le droit **équipe par équipe**, sur l'espace et la
  compétition de chacune — `teams` ne sait pas vérifier que la saison de la
  requête appartient à la compétition de la requête ;
- `post_update_competition` exige `est_admin` sur la compétition visée ;
- les widgets d'administration des poules et du calendrier passent par
  `require_admin_access`.

**Décisions du 2026-10-03, à appliquer** :
1. **Tout membre peut créer une compétition, et son créateur en devient
   automatiquement admin**, même sans l'avoir demandé. Toutes les étapes de
   l'assistant — et le widget de notifications de l'étape 4 — exigent ensuite
   `est_admin` sur la compétition.
2. **L'assistant est refusé une fois la compétition publiée** : les statuts
   `draft`, `rules_selected`, `structure_selected` et `invitations_configured`
   sont des brouillons ; à partir de `STATUT_SAISON_PRETE` (`ready`) et après,
   la compétition se modifie par ses panneaux d'administration. Le prédicat
   vit à côté de `STATUT_SAISON_PRETE`, pas dans les contrôleurs.
3. **Création de compte** : l'hôte injecte dans le contexte d'`auth` une
   fonction de droit, sur le modèle d'`ISpacesHostLayout`. La route n'a pas
   d'espace dans son chemin : le droit est « admin d'au moins un espace, ou le
   compte exploitant » — ceux qui voient le panneau aujourd'hui.

**Reste** : les décisions 1 à 3, les tests unitaires des gardes sur
`FakeAdminAccess`, les tests e2e par requête forgée.

## Tests

Unitaires : chaque garde refuse un simple membre et laisse passer un admin, sur
`FakeAdminAccess`.

E2E : pour chaque route, une requête forgée par un simple membre
(`X-Bypass-Auth-Profile: simple`) est refusée et ne change rien en base ; le
chemin nominal d'un admin passe toujours. Les tests existants d'inscription, de
création de compétition et de création de compte passent sans modification.

## Terminé quand

Aucune des routes du tableau n'accepte une requête d'un simple membre, et la
suite complète passe.
