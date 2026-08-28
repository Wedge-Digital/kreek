# Le formulaire et ses trois mutations

**Épic :** E10 — Référentiels éditables · **Ordre :** 9 · **Dépend de :** 467, 470
**Conception :** `docs/specs/competences-personnalisees/page-de-gestion/`
(`02-front.md`, `04-dtos.md`, `07-integration.md`)

## Objectif

Le widget de formulaire, ses trois modes, et les trois routes qui écrivent.

## Quatre routes

```
references/io/web/skill_admin/
├── skill_form.rs      GET    /app/{space_id}/admin/skills/form?skill_id=
└── skill_actions.rs   POST   /app/{space_id}/admin/skills
                       PUT    /app/{space_id}/admin/skills/{skill_uid}
                       DELETE /app/{space_id}/admin/skills/{skill_uid}
```

Ce sont les **trois premières mutations de `references`** — ses huit routes
actuelles sont toutes en `GET`.

## Trois modes dans une énumération

```rust
pub enum FormMode {
    Create,
    Edit { skill: SkillFormVm },
    EditLocked { skill: SkillFormVm, usage_count: u32 },
}
```

Et non deux booléens `is_edit` / `is_locked` : les trois états sont exclusifs, et
deux booléens en autorisent quatre — dont « création verrouillée », qui n'existe
pas.

| Mode | Nom | Description | Activation | Catégorie | Type |
|---|---|---|---|---|---|
| Création | libre | libre | libre | libre | libre |
| Édition, inemployée | libre | libre | libre | libre | libre |
| Édition, **employée** | libre | libre | libre | **figée** | **figé** |

### Un champ figé se transforme, il ne se grise pas

```
Catégorie   🔒 Agilité          ← un fait
Type        🔒 Élite            ← un fait
```

Et non un `<select>` désactivé. Griser inviterait à chercher comment réactiver
alors qu'il n'y a rien à réactiver.

### Le type dit ce qu'il coûte

Un segmenté « Standard / Élite », et choisir Élite affiche « +10 kPo à
l'achat ». C'est le seul champ à conséquence chiffrée : **la conséquence se voit
au moment de la décision**, pas quand un coach paie.

### L'aperçu de la pastille

Avec sa vraie teinte, celle de `components/skill-tints.css`. La voir ici évite de
découvrir après coup qu'elle se confond avec une autre.

### Les catégories sont statiques

`<kreek-select>` accepte des `<option>` statiques en alternative à son attribut
`url`. **Sept catégories immuables** : un endpoint JSON leur coûterait une route,
un contrôleur et un aller-retour pour une liste qu'Askama rend avec le
formulaire.

## Le contrôle d'accès

**Un résolveur `ISpaceOwnership` pour `skill_uid`**, dans
`infrastructure/references/space_ownership.rs` (fichier créé par la carte 441),
sur le modèle des six existants. Une compétence d'un autre espace rend alors
`404` **avant** le handler : la règle P2 devient structurelle plutôt que répétée.

**`belongs_to` reste dans le use case malgré tout** — non par défiance envers le
middleware, mais parce que le use case est testable unitairement et qu'une route
ajoutée plus tard hors du groupe scopé le laisserait nu.

## Les entrées

```rust
Form(payload): Form<CreateCustomSkillDto>
```

**`Form` et non `Json`** : six champs à plat. L'éditeur de roster prend du `Json`
parce que sa structure descend sur deux niveaux ; ici il n'y a rien à imbriquer.

**L'uid est dans le chemin, jamais dans le corps** — la leçon de la carte 416 :
`delete_match` et ses voisins prenaient leur cible dans le corps, hors de portée
de `space_scope`.

`UpdateCustomSkillDto` porte `category` et `skill_type` en `Option` : l'écran
verrouillé les rend en texte, donc le navigateur n'envoie rien. **Un `Option`
absent veut dire « inchangé » ; présent et différent est refusé par le use
case.** Le formulaire qui omet le champ est une commodité d'écran ; le contrôle
serveur est le garde, et seul le second tient face à un POST écrit à la main.

**Le middleware CSRF exige `HX-Request: true`** sur les trois verbes. Un `curl`
de mise au point échouera sans lui, et ça se cherche longtemps.

## Les sorties

| Cas | Réponse |
|---|---|
| `POST` réussi | le formulaire re-rendu **vide** + `HX-Trigger: customSkillsChanged` |
| `PUT` réussi | le formulaire re-rendu **en mode création** + le même `HX-Trigger` |
| `DELETE` réussi | `204` + le même `HX-Trigger` |
| `Invalid(DomainError)` | `422` + le formulaire, l'erreur nommant le champ |
| `NameTaken { name }` | `422` — le nom est dans le message |
| `SkillCategoryFrozen` / `SkillTypeFrozen` | **`409`** |
| `InUse { holders }` | **`409`** |
| `Forbidden` | `403` |
| `NotFound` | `404` |
| `UsageUnavailable` | **`503`** |

**`409` et non `422` pour les trois verrous** : la requête est bien formée, c'est
l'état du système qui s'y oppose. **`503` pour `UsageUnavailable`** : le refus est
temporaire, réessayer a du sens — ce qu'un `500` ne dit pas.

### Pourquoi le `PUT` rend le formulaire en mode création

Il pourrait rendre `204`, comme le `DELETE`. **Le formulaire resterait alors en
mode édition, affichant une compétence déjà enregistrée** — un état qui invite à
ré-enregistrer la même chose.

Le `POST`, lui, rend un formulaire **vide** : il est déjà en mode création. C'est
le geste réel — un organisateur qui écrit ses règles maison en saisit plusieurs
d'affilée.

## Ce qui reste front

**Rien de neuf.** Compteur de caractères, aperçu de pastille et bascule du type
sont de l'état d'écran dérivé des champs : un `x-data` inline suffit, là où
l'éditeur de roster demandait un fichier servi.

## Tests

| Test | Ce qu'il prouve |
|---|---|
| `le_formulaire_vide_est_en_mode_creation` | `FormMode::Create` |
| `le_formulaire_d_une_competence_libre_est_en_mode_edition` | `Edit` |
| `le_formulaire_d_une_competence_portee_fige_deux_champs` | `EditLocked` |
| `le_mode_verrouille_rend_la_categorie_en_texte` | pas un `select` désactivé |
| `le_post_rend_un_formulaire_vide_et_le_trigger` | l'enchaînement |
| `le_put_rend_le_formulaire_en_mode_creation` | la décision ci-dessus |
| `un_changement_de_categorie_sur_une_competence_portee_rend_409` | le code, pas 422 |
| `un_port_indisponible_rend_503` | le code, pas 500 |
| `une_competence_d_un_autre_espace_rend_404_avant_le_handler` | le résolveur |

## Checklist

- [ ] Les deux contrôleurs, les quatre routes
- [ ] Le résolveur `ISpaceOwnership` pour `skill_uid`
- [ ] `references-custom-skill-form.html`, les trois modes
- [ ] `<kreek-select>` à options statiques, aucun endpoint JSON
- [ ] Aucun `style="…"`, aucun `<select>` natif
- [ ] Les neuf tests
- [ ] `make lint && make test && make check-arch`
