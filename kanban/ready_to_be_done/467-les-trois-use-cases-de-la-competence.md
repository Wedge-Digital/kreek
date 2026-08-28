# Les trois use cases de la compétence personnalisée

**Épic :** E10 — Référentiels éditables · **Ordre :** 5
**Dépend de :** 464, 465, 466, 443
**Conception :** `docs/specs/competences-personnalisees/page-de-gestion/05-use-cases.md`

## Objectif

Créer, modifier, supprimer. Aucun écran — les trois se testent unitairement.

```
references/use_cases/
├── create_custom_skill_use_case.rs
├── update_custom_skill_use_case.rs
└── delete_custom_skill_use_case.rs
```

Le dossier arrive avec la carte 443.

## Le geste commun

```
1. l'appelant est-il admin de cet espace ?           → Forbidden
2. la compétence existe-t-elle, et dans CET espace ? → NotFound
3. combien de porteurs ?                             → selon la mutation
```

**`NotFound` et non `Forbidden` pour un espace étranger** : un `Forbidden`
confirmerait son existence à qui énumère. C'est la règle de `space_scope`, et
elle vaut d'autant plus ici que le contrôle est manuel — aucun de ses six
résolveurs ne connaît une compétence.

**La troisième étape est ce qui distingue cette fonctionnalité.** Pour un roster,
le compte d'usage interdisait la modification tout entière. Ici il ne l'interdit
pas : **il la rétrécit.**

## 1 · Création

```rust
pub async fn execute(
    cmd: CreateCustomSkillCommand,
    repo: &dyn IReferenceWriteRepository,
    admin: &dyn IReferencesSpaceAdminPort,
    id_service: &dyn IdService,
) -> Result<CustomSkillUid, CustomSkillError>
```

1. `is_space_admin` → `Forbidden`
2. la catégorie existe-t-elle au corpus ? → `Invalid`
3. le nom est-il libre dans la **liste fusionnée** ? → `NameTaken { name }`
4. engendrer `CUSTOM_<sulid>`
5. `CustomSkill::new(...)`, puis `to_reference_skill()`
6. `repo.save_custom_skill(...)`

**Aucun port d'usage** : une compétence qui naît n'a pas de porteur.

**La catégorie se vérifie**, même si le `<kreek-select>` ne propose que les sept :
un POST écrit à la main en enverrait une huitième, et la compétence serait
invisible dans tout sélecteur filtré par accès, peinte au repli `type-general`.
C'est la forme exacte de la carte 438 — la donnée entre, puis disparaît sans un
mot.

**L'unicité porte sur corpus + espace confondus**, rognés et insensibles à la
casse : c'est la liste que le coach voit dans un seul sélecteur. Deux « Esquive »,
l'une à 20 kPo l'autre à 30, est un piège pour qui choisit.

## 2 · Modification

```rust
pub async fn execute(
    cmd: UpdateCustomSkillCommand,
    repo: &dyn IReferenceWriteRepository,
    admin: &dyn IReferencesSpaceAdminPort,
    usage: &dyn ISkillUsagePort,
) -> Result<(), CustomSkillError>
```

1. accès, existence, `belongs_to`
2. **la commande demande-t-elle un changement risqué ?**
3. si oui seulement : `usage.count_usages(uid)`
4. `skill.amend(amendment, holders)` — **c'est l'agrégat qui tranche** (carte 464)
5. `repo.save_custom_skill(...)`, uid conservé

### Le compte ne se demande que s'il peut servir

```rust
let risque = cmd.category.is_some_and(|c| c != actuelle.category)
          || cmd.skill_type.is_some_and(|t| t != actuelle.skill_type);
```

Sans ce filtre, chaque correction de faute de frappe paierait un aller vers
`players_proj` — et pire, **échouerait dès qu'un joueur porte la compétence,
alors que rien de risqué n'était demandé.**

Quand `risque` est faux, `holders` vaut `Holders::new(0)` : l'agrégat n'a pas de
verrou à appliquer, et il n'a pas à savoir que le compte n'a pas été demandé.

### Le verrou se re-vérifie ici, pas seulement à l'écran

Entre l'affichage et l'enregistrement, un joueur peut acquérir la compétence.
**L'écran avertit, le serveur tranche.**

## 3 · Suppression

```rust
pub async fn execute(
    uid: &CustomSkillUid, space_id: &SpaceId,
    repo: &dyn IReferenceWriteRepository,
    admin: &dyn IReferencesSpaceAdminPort,
    usage: &dyn ISkillUsagePort,
) -> Result<(), CustomSkillError>
```

1. accès, existence, appartenance
2. `count_usages` — **inconditionnel**, c'est la question même
3. `skill.ensure_deletable(holders)` → `InUse { holders }`
4. `repo.delete_custom_skill(uid)`

**Aucun événement, et c'est démontrable.** Une compétence employée n'est pas
supprimable, et un roster qui la pose compte comme un usage : au moment où la
suppression réussit, **plus rien dans le système ne cite cet uid**. Rien à
nettoyer, personne à prévenir.

Cette carte n'a donc **pas besoin de la machinerie d'émission** que la carte 444
monte pour les rosters.

## Le doute ne ferme que la porte qu'il concerne

| Mutation | Port indisponible |
|---|---|
| création | sans objet |
| modification **sans** champ risqué | **passe** |
| modification **avec** champ risqué | `UsageUnavailable` |
| suppression | `UsageUnavailable` |

Traiter l'indisponibilité comme un zéro laisserait supprimer une compétence que
cent joueurs portent. Mais refuser une correction de faute de frappe pour la même
raison serait une sévérité gratuite : **la question n'était pas posée.**

## Les erreurs

```rust
pub enum CustomSkillError {
    Forbidden, NotFound,
    InUse { holders: u32 },
    ImmutableFieldChanged { field: &'static str },
    NameTaken { name: String },
    Invalid(DomainError),
    UsageUnavailable(String),
    Repository(String),
}
```

`InUse` et `ImmutableFieldChanged` sont **deux erreurs et non une**, bien que
nées du même compte : la première dit « on ne peut pas la supprimer », la seconde
« on ne peut plus changer ce champ ». Les fondre obligerait le contrôleur à
deviner le message d'après la route empruntée.

## Le cache n'apparaît pas ici

L'écriture en base puis le rafraîchissement du cache sont un détail
d'implémentation du dépôt (carte 465). **Le use case n'a pas à savoir qu'un cache
existe.**

## Instrumentation

`#[tracing::instrument(skip_all, fields(cmd = ?cmd))]` sur les trois — l'axe 11
l'exige. La suppression, qui prend des identifiants nus, nomme ses champs :
`fields(uid = ?uid, space_id = ?space_id)`.

## Tests

| Test | Règle |
|---|---|
| `un_non_admin_ne_cree_pas` | P1 |
| `une_competence_d_un_autre_espace_est_introuvable` | P2 |
| `une_categorie_inconnue_est_refusee` | C3 |
| `un_nom_deja_pris_au_corpus_est_refuse` | C6 |
| `un_nom_deja_pris_dans_l_espace_est_refuse` | C6 |
| `la_casse_et_les_espaces_ne_contournent_pas_l_unicite` | C6 |
| `corriger_un_libelle_ne_demande_pas_le_compte` | **U6** |
| `changer_la_categorie_demande_le_compte` | U6 a contrario |
| `un_port_indisponible_laisse_passer_une_correction_de_libelle` | **U7** |
| `un_port_indisponible_refuse_une_suppression` | U7 |
| `supprimer_une_competence_portee_est_refuse` | U1 |
| `l_uid_est_conserve_a_la_modification` | I3 |

`corriger_un_libelle_ne_demande_pas_le_compte` se vérifie sur un double de port
qui **panique** s'il est appelé : c'est le seul moyen de prouver une absence
d'appel.

## Checklist

- [ ] Les trois fichiers dans `references/use_cases/`
- [ ] `CustomSkillError`
- [ ] Le filtre `risque` avant tout appel au port
- [ ] Les trois `#[tracing::instrument]`
- [ ] Les douze tests
- [ ] `make lint && make test && make check-arch`
