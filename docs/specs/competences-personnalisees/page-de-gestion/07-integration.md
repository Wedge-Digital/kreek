# Page de gestion · Phase 7 : effets de bord

**Phase 6** : `06-domaine.md`

## Ce que cette phase révèle — l'inverse de ce qu'elle révélait pour les rosters

La phase 7 de l'éditeur de roster ouvrait sur un constat sévère : *« la moitié
du travail est ici, et elle est invisible depuis la maquette »*. `references`
n'avait ni couche applicative, ni ports, ni bus, ni infrastructure, ni table.

**Ici, tout cela existe déjà** — parce que les deux séries partent ensemble.

| Ce que `references` acquiert | Par qui |
|---|---|
| `use_cases/`, `ports.rs`, `domain/error.rs` | cartes 439-443 |
| `src/infrastructure/references/` | carte 441 |
| `IReferenceWriteRepository` et son cache | carte 441 |
| Un bus interne, un publisher, des app events | carte 444 |

Cette fonctionnalité ajoute **une table, trois méthodes de dépôt, six routes,
trois gabarits et une feuille**. Elle n'ajoute **aucune machinerie
d'événements** — c'est le second bénéfice de la livraison conjointe, après celui
que la phase 5 a démontré.

## Persistance

### La table

```sql
-- migrations/<date>_references_custom_skills.sql
CREATE TABLE references__custom_skills (
    uid         TEXT PRIMARY KEY,
    space_id    TEXT NOT NULL,
    name        TEXT NOT NULL,
    category    TEXT NOT NULL,
    skill_type  TEXT NOT NULL,
    activation  TEXT NOT NULL,
    description TEXT NOT NULL,
    created_by  TEXT NOT NULL,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at  TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX ON references__custom_skills (space_id);
```

**En colonnes, là où les rosters sont en JSONB** (phase 3) : une compétence est
six champs plats, et la catégorie se filtre.

**Aucun rattrapage.** La table naît vide, contrairement à `team_proj.roster_id`
qui devait rejouer l'event store.

### Le dépôt gagne trois méthodes

```rust
// s'ajoutent à IReferenceWriteRepository, créé par la carte 441
async fn save_custom_skill(&self, space_id: &SpaceId, skill: &Skill, by: &CoachId)
    -> Result<(), RepositoryError>;
async fn delete_custom_skill(&self, uid: &CustomSkillUid) -> Result<(), RepositoryError>;
async fn find_custom_skill(&self, uid: &CustomSkillUid)
    -> Result<Option<CustomSkillRecord>, RepositoryError>;
```

`save_custom_skill` est un `INSERT … ON CONFLICT (uid) DO UPDATE` : il sert la
création **et** la modification. Deux méthodes obligeraient le dépôt à
distinguer deux cas que le use case a déjà distingués.

`find_custom_skill` rend un **enregistrement** et non un `Skill` : le use case a
besoin du `space_id` pour l'appartenance, et le `Skill` du corpus ne le porte pas.

### Le cache, et son rafraîchissement

```rust
custom_skills:          RwLock<HashMap<String, Skill>>,      // uid → compétence
custom_skills_by_space: RwLock<HashMap<String, Vec<String>>>,// espace → uids
```

**Deux cartes et non une** : sans la seconde, lister les compétences d'un espace
demanderait de parcourir toute la première à chaque ouverture du sélecteur.

L'implémentation écrit en base **puis** rafraîchit les deux. Si le
rafraîchissement échoue, la base fait foi, la ligne part en `ERROR`, et un
redémarrage remet tout d'aplomb.

> **Le précédent à ne pas répéter** : la carte 362, « le bundle CSS est gelé au
> démarrage » — un cache que rien ne rafraîchit et dont l'obsolescence ne se
> signale pas.

### Deux signatures de lecture changent

**`find_skill_by_uid` rend `Option<Skill>` au lieu de `Option<&Skill>`** — sept
sites d'appel, tous en retirant un `&` :

```
consistency.rs:82                     skill_catalog_adapter.rs:46
reference_data_adapter.rs:43, :79, :107, :118
roster_catalog_adapter.rs:23
```

**`list_skills_for_space` naît à côté de `list_skills`**, qui reste au corpus
seul. Trois appelants (phase 4) :

| Appelant | État |
|---|---|
| `skill_picker.rs:158` | existant — et sa route change, voir ci-dessous |
| `skill_catalog_adapter.rs:52` | existant |
| l'éditeur de roster (carte 446) | à naître, écrit d'emblée avec l'espace |

### La route du sélecteur gagne son espace

```
/references/roster-lines/skill-picker
→ /app/{space_id}/references/roster-lines/skill-picker
```

**Un seul consommateur** : `team_creation/io/web/templates/finalize-team.html:81`,
via `app_routes.references.skill_picker_base()`. Le gabarit connaît son
`space_id`, le changement est mécanique.

Ce que ça apporte au-delà du besoin : le sélecteur entre sous `space_scope`,
alors qu'il sert aujourd'hui le catalogue à qui le demande.

## Événements

**Aucun.** Ni domain event, ni app event, ni publisher, ni listener.

La démonstration est en phase 5 : une compétence employée n'est pas supprimable,
et un roster qui la pose compte comme un usage — donc au moment où la
suppression réussit, plus rien dans le système ne cite cet uid. **Rien à
nettoyer, personne à prévenir.**

C'est l'écart le plus net avec les rosters, où `CustomRosterDeleted` doit
traverser publisher et listener pour retirer l'uid des tiers de compétition.

## Contrôle d'accès — un résolveur, et une vérification qui reste

`space_scope` couvre `{space_id}` mais **pas** `{skill_uid}` : aucun de ses six
résolveurs ne connaît une compétence (phase 4).

**1. Un résolveur `ISpaceOwnership` pour `skill_uid`**, dans
`infrastructure/references/space_ownership.rs` — fichier créé par la carte 441
pour `roster_uid`, sur le modèle des six existants. Une compétence d'un autre
espace rend alors `404` **avant** le handler : la règle P2 devient structurelle
plutôt que répétée.

**2. `belongs_to` reste dans le use case malgré tout.** Non par défiance envers
le middleware, mais parce que le use case est testable unitairement et que le
middleware ne l'est pas — et surtout parce qu'une route ajoutée plus tard hors
du groupe scopé laisserait le use case nu. Le contrôle qui vit dans la fonction
suit la fonction.

**3. `is_space_admin` en première ligne des trois mutations** (P1). Les lectures,
elles, restent aux seuls administrateurs : la page entière leur est réservée,
contrairement à la liste des rosters que tout membre consulte pour choisir.

## Handlers

```
references/io/web/skill_admin/
├── mod.rs
├── skill_admin_page.rs     GET    /app/{space_id}/admin/skills
├── skill_form.rs           GET    /app/{space_id}/admin/skills/form
├── skill_rows.rs           GET    /app/{space_id}/admin/skills/list
└── skill_actions.rs        POST   /app/{space_id}/admin/skills
                            PUT    /app/{space_id}/admin/skills/{skill_uid}
                            DELETE /app/{space_id}/admin/skills/{skill_uid}
```

Six routes. `references/routes.rs` en compte huit aujourd'hui, **toutes en
`GET`** : ce sont ses trois premières mutations, avec celles du roster.

```rust
pub async fn post_skill(
    auth_session: AuthSession,
    Path(space_id): Path<String>,
    State(state): State<AppState>,
    Form(payload): Form<CreateCustomSkillDto>,
) -> Response;
```

**`Form` et non `Json`** : six champs à plat (phase 4). L'éditeur de roster prend
du `Json` parce que sa structure descend sur deux niveaux ; ici il n'y a rien à
imbriquer.

**L'uid de la compétence est dans le chemin, jamais dans le corps.** C'est la
leçon de la carte 416 — `delete_match` et ses voisins prenaient leur cible dans
le corps, hors de portée de `space_scope`. Ici le résolveur peut faire son
travail.

`skill_form.rs` prend `?skill_id=` en `Query` : c'est un paramètre d'affichage,
pas une ressource à scoper, et son absence est le mode création.

**Le middleware CSRF exige `HX-Request: true`** sur `POST`, `PUT` et `DELETE`.
Les trois mutations partent d'attributs `hx-*`, donc l'en-tête y est — mais un
`curl` de mise au point échouera sans lui, et ça se cherche longtemps.

### Les sorties

| Cas | Réponse |
|---|---|
| `POST` réussi | le formulaire re-rendu **vide** + `HX-Trigger: customSkillsChanged` |
| `PUT` réussi | le formulaire re-rendu **en mode création** + le même `HX-Trigger` |
| `DELETE` réussi | `204` + le même `HX-Trigger` |
| `Invalid(DomainError)` | `422` + le formulaire, l'erreur nommant le champ |
| `NameTaken { name }` | `422` — et le nom est dans le message |
| `SkillCategoryFrozen` / `SkillTypeFrozen` | **`409`** |
| `InUse { holders }` | **`409`** |
| `Forbidden` | `403` |
| `NotFound` | `404` |
| `UsageUnavailable` | **`503`** |

**`409` et non `422` pour les trois verrous** : la requête est bien formée, c'est
l'état du système qui s'y oppose. Et **`503` pour `UsageUnavailable`** : le refus
est temporaire, réessayer a du sens — ce qu'un `500` ne dit pas.

### Pourquoi le `PUT` rend le formulaire en mode création

Il pourrait rendre `204`, comme le `DELETE`. **Le formulaire resterait alors en
mode édition, affichant une compétence déjà enregistrée** — un état qui invite à
ré-enregistrer la même chose, et où le bouton dit « Enregistrer » sans qu'il y
ait rien à enregistrer.

Le `POST`, lui, rend un formulaire **vide et non réinitialisé au mode création** :
il y est déjà. C'est le geste réel — un organisateur qui écrit ses règles
maison en saisit plusieurs d'affilée (phase 2).

## Templates et CSS

```
references/io/web/templates/
├── references-custom-skills.html        ← la page hôte, deux conteneurs hx-get
├── references-custom-skill-form.html
└── references-custom-skill-list.html
```

Une feuille, `pages/references-custom-skills.css`, portée par `.cs-page`,
**inscrite dans `src/web/css_bundle.rs`** parmi les pages — l'axe 14 refuse une
feuille absente du bundle.

**Aucun JS de composant.** Le compteur de caractères, l'aperçu de pastille et la
bascule du type sont de l'état d'écran dérivé des champs (phase 2) : un `x-data`
inline suffit, là où l'éditeur de roster demandait un fichier servi.

### Les pastilles de catégorie vont poser un problème de contrôle

Les sept teintes vivent dans `widgets/players-widget.css:34-41`. La page les
emploie **sans les redéfinir** — les redéfinir ferait deux jeux qui dérivent,
une compétence verte ici et bleue sur la fiche du joueur.

**Mais `tests/e2e/visual/debordements.py` pose exactement cette question** :

> ce sélecteur trouve-t-il du markup sur une page qui ne chargeait pas sa
> feuille ?

`widgets/players-widget.css` trouvera `.type-agility` sur la page des
compétences. **Le contrôle signalera un débordement**, et il aura formellement
raison.

La sortie n'est pas une exception : **les sept teintes montent dans
`components/skill-tints.css`**. Les feuilles de `components/` sont globales par
construction dans ce contrôle, donc le débordement disparaît de lui-même, et les
deux pages consomment la même définition.

C'est **la même décision que celle de la phase 4**, appliquée à l'autre couche :
là-bas `references` devenait propriétaire de `category_css` côté Rust, ici la
teinte devient un composant côté CSS. Les faire séparément serait s'arrêter à
mi-chemin.

## Tests E2E

`tests/e2e/test_custom_skills.py`.

| Scénario | Ce qu'il prouve |
|---|---|
| `test_creer_une_competence` | le chemin heureux, formulaire vidé, liste rechargée |
| `test_la_competence_creee_apparait_dans_le_selecteur_de_spp` | **le test qui vaut le prix de la suite** |
| `test_un_nom_deja_pris_est_refuse` | C6, corpus compris |
| `test_un_nom_avec_apostrophe_passe` | `TEXTE_SAISI`, bout en bout |
| `test_une_competence_portee_garde_ses_champs_de_libelle_ouverts` | U2 |
| `test_une_competence_portee_affiche_sa_categorie_en_texte` | elle n'est pas grisée, elle est un fait |
| `test_le_bouton_supprimer_n_existe_pas_sur_une_competence_portee` | absent, pas désactivé |
| `test_corriger_le_nom_d_une_competence_portee_reussit` | **U6, le piège** |
| `test_un_non_admin_ne_voit_pas_la_page` | P1 |
| `test_la_competence_d_un_autre_espace_rend_404` | P2, via le résolveur |
| `test_le_type_elite_coute_dix_kpo_de_plus` | C4, jusqu'au barème |

**`test_la_competence_creee_apparait_dans_le_selecteur_de_spp`** traverse tout :
l'écriture en base, le rafraîchissement des deux cartes, l'aiguillage par
préfixe dans `find_skill_by_uid`, `list_skills_for_space`, la route du sélecteur
qui a gagné son `space_id`. **C'est aussi le seul qui prouve que le cache n'est
pas gelé** — sans redémarrage entre la création et la vérification.

**`test_corriger_le_nom_d_une_competence_portee_reussit`** est le pendant e2e du
test unitaire de la phase 6. Sans lui, la suite est verte sur une fonctionnalité
où personne ne peut plus corriger une faute de frappe : les refus refusent, les
créations créent, et le seul chemin cassé est celui qu'on n'a pas pensé à
parcourir.

**`test_le_type_elite_coute_dix_kpo_de_plus`** est le seul qui atteigne
l'argent. Il crée une compétence Élite, la fait acheter en SPP, et vérifie le
débit — c'est ce qui attraperait un `"Elite"` sans accent, qu'aucun test de
sérialisation ne verrait si quelqu'un corrigeait le test plutôt que le type.

**Aucun `sleep`.** `cliquer_quand_cable` pour tout contenu fraîchement injecté :
les deux widgets se rechargent sur `customSkillsChanged`, donc chaque action qui
suit une mutation tombe dans la fenêtre où l'élément est peint mais pas câblé.

## Ce que la phase ne prévoit pas

- **Aucune duplication d'une compétence du règlement** pour la retoucher (phase 2).
- **Aucun partage entre espaces.**
- **Aucune traduction** : la compétence est saisie dans la langue de son auteur.
- **Aucun test de charge** : une poignée de compétences par espace.

## Règles métier

**Aucune à préciser.** Les vingt et une de la phase 6 couvrent la fonctionnalité.
Cette phase décrit des mécanismes, et le seul arbitrage qu'elle tranche — la
sortie du `PUT` — est une question d'écran, pas de règle.
