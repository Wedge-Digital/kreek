# Phase 4 — Contrats de données : l'écran de réglage des notifications

**Entrée** : `03-back.md`, validée.

## Convention suivie

Dans `competitions`, les booléens de domaine sont des **newtypes nus**
(`pub struct UseSchedule(pub bool)`, `#[serde(transparent)]`) ; `nutype` y est
réservé aux types à contrainte — un booléen n'a rien à valider. Les quatre
réglages suivent cette maison plutôt que d'en ouvrir une seconde.

## Domaine — `competition_notifications.rs`

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct NotifyRegistrationOpen(pub bool);      // ouverture des inscriptions

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct NotifyRoundEve(pub bool);              // veille de journée

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct NotifyRoundClosing(pub bool);          // fin de journée imminente

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct NotifyRegistrationDeadline(pub bool);  // date limite d'inscription

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompetitionNotifications {
    #[serde(default = "actif")]
    pub registration_open: NotifyRegistrationOpen,
    #[serde(default = "actif")]
    pub round_eve: NotifyRoundEve,
    #[serde(default = "actif")]
    pub round_closing: NotifyRoundClosing,
    #[serde(default = "actif")]
    pub registration_deadline: NotifyRegistrationDeadline,
}
```

**Émis par** : le repository (lecture de la colonne), le handler du POST
(construction). **Consommé par** : la fonction d'applicabilité, le use case, le
constructeur du VM.

### Le défaut vaut « tout allumé », et c'est la migration qui rend cela possible

R8 veut les saisons existantes éteintes et les neuves allumées. Ces deux défauts
ne coexistent que si **la migration écrit explicitement les quatre `false` sur
toutes les lignes présentes** le jour où elle passe.

Sans ce remplissage, `NULL` signifierait deux choses opposées — « ancienne
saison, donc éteint » et « saison neuve, donc allumé » — sans rien dans la ligne
pour les distinguer. Le `status` n'y suffit pas : `invitations_configured`
désigne aussi bien une saison abandonnée en cours de magicien qu'une saison
d'avant la migration.

Une fois les lignes remplies, `NULL` retrouve un sens unique — « créée après la
migration » — et `#[serde(default)]` peut valoir « allumé » sans ambiguïté.

## Domaine — l'applicabilité (R5)

```rust
/// Pourquoi une notification ne peut pas se déclencher sur cette saison.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Inapplicable {
    NoSchedule,        // la compétition n'a pas de calendrier
    NoTimeFrameRound,  // aucune journée n'a de fenêtre à clore
    NoDeadline,        // aucune date limite d'inscription n'est fixée
}

/// `None` = applicable. `registration_open` n'y figure pas : elle l'est
/// toujours, une compétition ayant par construction une ouverture.
pub struct NotificationApplicability {
    pub round_eve: Option<Inapplicable>,
    pub round_closing: Option<Inapplicable>,
    pub registration_deadline: Option<Inapplicable>,
}

pub fn applicability(
    structure: &CompetitionStructure,
    invitations: Option<&CompetitionInvitations>,
) -> NotificationApplicability;
```

**Émis par** : la fonction de domaine. **Consommé par** : le handler du GET,
pour bâtir le VM. Jamais par un template — un template ne lit pas un enum de
domaine.

La fonction prend les deux : les motifs de journée viennent de la structure, le
motif de date limite des invitations. C'est la seule raison pour laquelle le
handler du GET charge deux colonnes.

## Entrée — DTO de transport du POST

```rust
#[derive(Debug, Deserialize)]
pub struct NotificationSettingsPayload {
    pub registration_open: bool,
    pub round_eve: bool,
    pub round_closing: bool,
    pub registration_deadline: bool,
}
```

**Émis par** : le widget en mode auto-save. **Consommé par** : le handler du
POST, qui bâtit `CompetitionNotifications` — donc les value objects — avant
d'appeler le use case.

> **Corrigé en phase 7** : ce DTO arrive en **encodage de formulaire**, pas en
> JSON — `hx-post` poste un formulaire. Une case non cochée n'étant alors pas
> envoyée du tout, ses quatre champs portent `#[serde(default)]` et l'extracteur
> est `Form`. Détail et symptôme dans `07-integration.md`.

**Un DTO de transport, et non la struct de domaine.** L'étape 4 actuelle fait
`Json(invitations): Json<CompetitionInvitations>` : la struct de domaine *est* le
format de fil. C'est commode et c'est à l'envers — le handler doit construire la
commande, value objects compris. Le nouveau POST ne reprend pas ce pli ;
l'existant n'est pas touché au passage.

## Entrée — la voie du magicien

En mode différé, le widget ne POSTe pas : la page hôte fusionne l'événement dans
son `state` et l'envoie avec le reste de l'étape 4. Le corps existant gagne donc
un sous-objet :

```jsonc
{
  "access_mode": "...", "requires_validation": true, "invited_coaches": [...],
  "max_participants": null, "registration_deadline": null,
  "notifications": { "registration_open": true, "round_eve": true,
                     "round_closing": true, "registration_deadline": true }
}
```

`notify_by_email` **disparaît** de ce corps : les quatre réglages le remplacent.

### La page hôte doit réhydrater son `state`, comme elle le fait déjà

Le gabarit de l'étape 4 reçoit un `existing_notifications_json` en plus de
`existing_invitations_json`, et en initialise `state.notifications`.

**Ce n'est pas un ajout, c'est le maintien de ce que la page fait déjà** —
`state.notifyByEmail = INITIAL.notify_by_email …`. Déplacer la case dans un
widget déplace son rendu, pas la réhydratation de l'hôte : le widget affiche
alors les valeurs sauvegardées pendant que `state` porte le défaut, et une
re-validation sans toucher aux cases écrase les réglages.

Ce JSON **et** l'émission à l'`init()` du widget (phase 2) ont deux rôles
distincts, et aucun ne remplace l'autre :

| Mécanisme | Ce qu'il garantit |
|---|---|
| `existing_notifications_json` | `state` est juste **dès la première peinture**, sans dépendre du `hx-get` du widget |
| émission à l'`init()` | l'hôte suit le widget si celui-ci a lu plus tard, et le contrat reste « voici l'état » |

Sans le premier, valider pendant que le widget charge encore rejouerait le
défaut. Sans le second, l'hôte dépendrait d'un JSON que seul le magicien reçoit,
et le widget ne serait plus autonome.

`SaveCompetitionInvitationsCommand` gagne un champ `notifications:
CompetitionNotifications`, et son use case écrit **les deux colonnes**. Un seul
use case pour un seul handler, plutôt que deux appels à orchestrer — et les deux
écritures peuvent partager une transaction, ce que deux appels ne garantiraient
pas.

## Sortie — les view models

```rust
pub struct NotificationRowVm {
    /// Le nom du champ dans l'événement DOM et dans le payload — « round_eve ».
    /// C'est aussi la clé sur laquelle Alpine reconnaît la ligne à griser.
    pub key: &'static str,
    pub label: String,
    pub description: String,
    /// « la veille du début de chaque journée »
    pub when: String,
    pub checked: bool,
    /// `None` = applicable.
    ///
    /// **Pour la ligne `registration_deadline`, ce n'est qu'un état de départ**,
    /// écrasé par Alpine dès la première frappe dans le champ de date de
    /// l'étape 4. Ne jamais s'en servir comme d'une vérité côté serveur.
    pub inapplicable_reason: Option<String>,
}

pub struct NotificationSettingsVm {
    pub rows: Vec<NotificationRowVm>,
    /// « deferred » | « autosave » — pilote la pose des attributs HTMX.
    pub mode: &'static str,
    /// Vide en mode différé, où le widget ne POSTe pas.
    pub post_url: String,
    /// Le motif qu'Alpine affiche quand l'utilisateur efface la date limite.
    /// Présent **même quand la ligne démarre applicable** : sinon le client
    /// n'aurait rien à afficher au moment où il en a besoin.
    pub deadline_cleared_reason: String,
}
```

**Émis par** : `NotificationSettingsVm::from_domain(&notifications,
&applicability, mode, post_url)` — constructeur co-localisé, puisque le VM ne
dépend que du domaine et d'aucun DTO de port. **Consommé par** : le template
`notification-settings-widget.html`, et par l'Alpine du widget via les attributs
qu'il rend.

`deadline_cleared_reason` est le seul champ que le serveur envoie pour un cas
qui n'existe pas encore à l'instant du rendu. Il est là parce que le client doit
pouvoir expliquer le grisage sans aller-retour — et qu'un libellé métier n'a rien
à faire en dur dans du JavaScript.

## Interfaces — qui émet, qui consomme

| DTO | Émetteur | Consommateur |
|---|---|---|
| `CompetitionNotifications` | repository (lecture), handler POST (écriture) | applicabilité, use case, `from_domain` |
| `NotificationApplicability` | fonction de domaine | handler GET |
| `NotificationSettingsPayload` | Alpine du widget (auto-save) | handler POST |
| sous-objet `notifications` du corps de l'étape 4 | page hôte du magicien | handler POST de l'étape 4 |
| `NotificationSettingsVm` / `NotificationRowVm` | `from_domain()` | template du widget |

Aucun DTO de port : `competitions` possède les saisons.

## Pas de verrou optimiste

Quatre booléens, dernier écrivain gagnant. La colonne séparée décidée en phase 3
suffit à protéger ce qui devait l'être — les invitations concurrentes — et un
numéro de version pour quatre cases coûterait plus qu'il ne rapporte.

## Règles métier

### R8 — apparue à cette phase, tranchée

Les saisons existantes démarrent **tout éteint**, les nouvelles **tout allumé**.
Consignée dans le README ; sa conséquence technique — le remplissage explicite
par la migration — est décrite plus haut et revient en phase 7.

## Ce que cette phase laisse aux suivantes

- **Phase 5** — le use case de sauvegarde, et l'extension de celui de l'étape 4.
- **Phase 6** — l'écriture de `applicability()`, seule vraie logique métier de
  cet écran.
- **Phase 7** — la migration : ajout de colonne, remplissage des lignes
  existantes, retrait des deux interrupteurs morts de leurs blobs respectifs.
