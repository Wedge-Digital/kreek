# Le panneau de modification de la trésorerie

**Ordre :** 3 · **Dépend de :** `558`
**Conception :** `docs/specs/ajustement-tresorerie/panneau-d-ajustement/`
(`02-front.md`, `03-back.md`, `04-dtos.md`, `07-integration.md`)
**Maquette :** `assets/rawpages/html/app-team-treasury-admin.html`

## Objectif

Le bouton, le panneau, la route et sa réponse. C'est la carte où la
fonctionnalité devient visible — et la plus grosse des quatre.

## Conception

### 1. Qui voit le bouton — `team_detail.rs`

Dans `rendre_fiche`, à côté du `peut_editer` existant :

```rust
let est_admin = match auth_session.user.as_ref() {
    Some(user) => state.teams.access_port.is_space_admin(&user.id, &space_id).await,
    None => false,
};
```

**Le port, et pas l'extracteur `SpacePermissions`** : celui-ci répond 403 à un
non-membre, et fermerait un relevé qu'on veut lisible par tous. `ITeamAccessPort`
est déjà câblé et déjà appelé dix lignes plus haut.

`rendre_fiche` gagne le paramètre, le passe à `contenu_de_l_onglet`, qui le
passe à `rendre_onglet` avec `space_id`. **Les deux autres onglets passent
`false`** — un mot chacun, aucun changement de comportement.

Coût connu et assumé : `peut_modifier_effectif` interroge déjà `is_space_admin`
pour le même couple, le port sera donc appelé deux fois par rendu. Les
factoriser demanderait de séparer admin et propriétaire dans ce service, qui les
mêle en un booléen — hors sujet ici.

### 2. Les view models

```rust
pub struct TreasuryVm {
    …,
    pub adjust: Option<AdjustPanelVm>,   // None = le visiteur n'est pas admin
}

pub struct AdjustPanelVm {
    pub post_url:    String,
    pub balance_kpo: u32,
    pub min_kpo:     u32,   // 5
    pub step_kpo:    u32,   // 5
    pub max_kpo:     u32,   // 500
}

pub struct AdjustErrorVm { pub message: String }
```

**`Option` et non un booléen** : pour un non-admin il n'y a pas de panneau, pas
un panneau vide. Le gabarit ne peut alors pas rendre un formulaire sans son URL.

**Les bornes descendent dans le VM**, jamais écrites en dur dans le gabarit —
sinon 500 vivrait à deux endroits, et le jour où le plafond change l'un des deux
serait oublié. Règle « un gabarit n'invente aucune valeur ».

### 3. La route

```
POST /app/{space_id}/teams/{team_id}/tresorerie/ajuster
```

Chemin en français, comme `TEAM_TREASURY` qu'il prolonge. **Dans
`routes_ouvertes()`** : le groupe `routes_d_action()` porte `garde_action_equipe`,
qui est la règle du coach — un commissaire n'en est pas un, et le groupe le
refuserait avant le handler. `DISMISS_TEAM` est déjà dans ce cas.

Ne pas oublier **l'entrée du test d'inventaire** de `routes.rs` (l. 279-316).

### 4. Le contrôleur — `adjust_treasury_controller.rs`

```rust
pub async fn adjust_treasury(
    Path((space_id, team_id)): Path<(String, String)>,
    perms: SpacePermissions,
    State(state): State<AppState>,
    auth_session: AuthSession,
    Form(form): Form<AdjustTreasuryForm>,   // en dernier — axum l'exige
) -> Response
```

`403` si `!perms.is_admin()`, comme `dismiss_team.rs:18`. Découpé en
`construire_commande` et `repondre_erreur` pour tenir sous vingt lignes.

| Cas | Réponse |
|---|---|
| pas admin | **403** |
| sens illisible | **400** — pas une faute d'usager, donc pas de message à montrer |
| montant ou motif invalide | **200** + pied d'erreur |
| `InsufficientTreasury` | **200** + pied d'erreur |
| `ConcurrentWrite` | **200** + pied d'erreur, « la fiche a changé » |
| équipe introuvable | **404** |
| autre erreur de dépôt | **500**, journalisée |

**200 et non 4xx sur les refus de saisie** : htmx n'échange pas une réponse
non-2xx par défaut. Un 422 n'afficherait rien, et le bouton paraîtrait sans
effet.

**Le succès rend la page entière**, par `rendre_fiche(…, "treasury")` — rien de
neuf à écrire.

### 5. Les en-têtes d'erreur

```
HX-Retarget: #adm-foot    HX-Reselect: #adm-foot    HX-Reswap: outerHTML
```

**`HX-Reselect` n'est pas une précaution.** Le formulaire porte `hx-select`, qui
filtre aussi la réponse d'erreur, n'y trouve pas `#app-content`, et **rien ne
s'affiche**. `finalize_team.rs:44-50` documente ce piège pour l'avoir payé.

### 6. Le gabarit

Dans `teams-treasury-tab.html`, sous `{% if let Some(adjust) = vm.adjust %}` : le
bouton dans le bandeau de synthèse, le panneau sous lui.

```html
hx-post="{{ adjust.post_url }}"
hx-target="#app-content" hx-select="#app-content" hx-swap="outerHTML"
```

**`#app-content` et non `#team-tab-zone`** : la trésorerie s'affiche **deux
fois**, dans le bandeau et dans l'en-tête de fiche — et l'en-tête est hors de la
zone d'onglets. Un swap limité à la zone laisserait deux chiffres contradictoires
à l'écran.

Le pied part dans un partiel, rendu dans la page **et** seul en cas d'erreur.
Askama résout `{% include %}` contre le contexte du gabarit incluant : si les
noms de champs ne concordent pas, **assumer six lignes dupliquées** plutôt que
tordre les VMs.

Pas d'`hx-disinherit` : vérifié, ni la fiche ni le layout ne posent d'attribut
héritable.

### 7. Alpine et CSS

Un `x-data` sur la racine du panneau : ouverture, sens, annonce vivante du
nouveau solde, et les refus dans l'ordre où ils bloquent — positif, multiple de
5, plafonné, couvert par le solde, motif saisi.

**Alpine et non un `<script>` nu** : le panneau est remplacé à chaque swap, et un
`x-data` se ré-initialise seul là où un script survivrait au remplacement du DOM.

Les règles vont dans `pages/team-treasury.css`, déjà au bundle et portée
`.team-treasury`. Pas de feuille neuve : elle n'aurait pas de racine à elle.

Sous 768 px : bouton pleine largeur, formulaire et pied en colonne.

## Checklist

- [ ] `est_admin` dans `rendre_fiche`, descendu jusqu'à `rendre_onglet` avec `space_id`
- [ ] Les deux autres onglets passent `false`
- [ ] `AdjustPanelVm`, `AdjustErrorVm`, `TreasuryVm.adjust: Option<…>`
- [ ] Route, router, **et l'entrée du test d'inventaire**
- [ ] `AdjustTreasuryForm` + contrôleur sous vingt lignes
- [ ] Les trois en-têtes d'erreur, `HX-Reselect` compris
- [ ] Bouton et panneau dans `teams-treasury-tab.html`
- [ ] Partiel du pied, rendu dans la page et seul
- [ ] `x-data` Alpine : ouverture, sens, annonce, cinq refus
- [ ] CSS dans `pages/team-treasury.css`, responsive 768 px
- [ ] Test de handler : 403 pour un non-admin, et le markup du panneau absent du GET
