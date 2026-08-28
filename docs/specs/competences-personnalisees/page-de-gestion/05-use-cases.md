# Page de gestion · Phase 5 : use cases

**Phase 4** : `04-dtos.md`

## Trois mutations, trois use cases

```
references/use_cases/
├── create_custom_skill_use_case.rs
├── update_custom_skill_use_case.rs
└── delete_custom_skill_use_case.rs
```

**Le dossier arrive avec l'épic E10** (carte 443) : `references` n'a jamais eu de
couche applicative, il servait des données lues au démarrage sans une seule
écriture. Les compétences s'y installent, elles ne le fondent pas.

## Le geste commun — et là où il diverge de celui des rosters

```
1. l'appelant est-il admin de cet espace ?          → Forbidden
2. la compétence existe-t-elle, et dans CET espace ? → NotFound
3. combien de porteurs a-t-elle ?                    → selon la mutation
```

**`NotFound` et non `Forbidden` pour une compétence d'un autre espace.** Un
`Forbidden` confirmerait son existence à qui énumère — c'est la règle que
`space_scope` applique déjà, et elle vaut ici d'autant plus que le contrôle est
manuel : aucun de ses six résolveurs ne connaît une compétence (phase 4).

**La troisième étape est ce qui distingue cette fonctionnalité.** Pour un roster,
le compte d'usage **interdisait la modification tout entière**. Ici il ne
l'interdit pas : il la **rétrécit**. Le nom, la description et l'activation
passent toujours ; seuls la catégorie et le type se ferment.

## 1 · `create_custom_skill_use_case`

```rust
pub async fn execute(
    cmd: CreateCustomSkillCommand,
    repo: &dyn IReferenceWriteRepository,
    admin: &dyn IReferencesSpaceAdminPort,
    id_service: &dyn IdService,
) -> Result<CustomSkillUid, CustomSkillError>
```

1. `admin.is_space_admin(...)` → `Forbidden`
2. la catégorie existe-t-elle au corpus ? → `Invalid`
3. le nom est-il libre dans l'espace ? → `NameTaken { name }`
4. engendrer `CUSTOM_<sulid>`
5. convertir en `Skill`
6. `repo.save_custom_skill(space_id, &skill, created_by)`

**Aucun port d'usage.** Une compétence qui naît n'a pas de porteur — poser la
question serait un aller-retour pour une réponse connue.

**Aucun événement**, et cette fois pour une raison plus forte que celle des
rosters : voir « La suppression n'émet rien » ci-dessous.

### La catégorie se vérifie, même si le `<kreek-select>` ne propose que les sept

Le sélecteur est rendu par le serveur avec les sept catégories du corpus. Un
POST écrit à la main en enverrait une huitième, et le catalogue accueillerait
une compétence dont la catégorie ne correspond à rien — invisible dans tout
sélecteur filtré par accès, et peinte au repli `type-general`.

C'est la forme exacte de la carte 438 : **la donnée entre, puis disparaît sans
un mot.**

## 2 · `update_custom_skill_use_case`

```rust
pub async fn execute(
    cmd: UpdateCustomSkillCommand,
    repo: &dyn IReferenceWriteRepository,
    admin: &dyn IReferencesSpaceAdminPort,
    usage: &dyn ISkillUsagePort,
) -> Result<(), CustomSkillError>
```

1. accès, existence, appartenance à l'espace
2. **la commande demande-t-elle un changement risqué ?**
3. si oui seulement : `usage.count_usages(uid)` → si > 0,
   `ImmutableFieldChanged { field }`
4. `repo.save_custom_skill(...)`, uid conservé

### Le compte ne se demande que s'il peut servir

```rust
let risque = cmd.category.is_some_and(|c| c != actuelle.category)
          || cmd.skill_type.is_some_and(|t| t != actuelle.skill_type);
```

**`Some` ne suffit pas — il faut `Some` et différent.** Un écran déverrouillé
renvoie toujours les deux champs, à leur valeur d'origine dans le cas courant.
Traiter leur présence comme une demande de changement ferait payer un aller-vers
`players_proj` à chaque correction de faute de frappe, et pire : ferait échouer
la correction dès qu'un joueur porte la compétence, alors que **rien de risqué
n'était demandé**.

C'est la différence de fond avec l'éditeur de roster, où le compte était
inconditionnel parce que toute modification était bloquée.

### Le verrou se re-vérifie ici, pas seulement à l'écran

L'écran affiche les champs ouverts sur un compteur à zéro. Entre l'affichage et
l'enregistrement, un joueur peut acquérir la compétence. **Le use case
recompte** : l'écran avertit, le serveur tranche.

### Ce que le refus doit dire

`ImmutableFieldChanged { field }` nomme le champ. « La catégorie ne peut plus
changer : trois joueurs ont payé le barème de Force » se comprend ; « modification
refusée » envoie chercher.

**Et le refus est total.** On n'enregistre pas le nom en écartant la catégorie :
l'administrateur croirait son changement passé. Une écriture partielle silencieuse
est pire qu'un refus, et c'est ce qui a valu la carte 427.

## 3 · `delete_custom_skill_use_case`

```rust
pub async fn execute(
    uid: &CustomSkillUid,
    space_id: &SpaceId,
    repo: &dyn IReferenceWriteRepository,
    admin: &dyn IReferencesSpaceAdminPort,
    usage: &dyn ISkillUsagePort,
) -> Result<(), CustomSkillError>
```

1. accès, existence, appartenance
2. `usage.count_usages(uid)` → si > 0, `InUse { holders }`
3. `repo.delete_custom_skill(uid)`

**Ici le compte est inconditionnel** : c'est la question même de la suppression.

### La suppression n'émet rien — et c'est démontrable

`delete_custom_roster_use_case` émet `CustomRosterDeleted`, que `competitions`
écoute pour nettoyer ses tiers. **Rien d'équivalent ici**, et pas par
négligence :

> Une compétence employée n'est pas supprimable — et un roster qui la pose en
> compétence de base compte comme un usage (phase 3).

Donc au moment où la suppression réussit, **aucune donnée du système ne cite
cet uid**. Il n'y a rien à nettoyer, donc personne à prévenir.

C'est le bénéfice concret du second compte de la phase 3 : il coûte une requête,
et il épargne un domain event, un app event, un publisher et un listener.

**Cette fonctionnalité n'a donc pas besoin de la machinerie d'émission** que la
carte 444 monte pour les rosters — « la moitié du travail de cette carte », et
elle ne la paie pas deux fois.

## Le doute ne ferme que la porte qu'il concerne

```rust
UsageUnavailable(String)
```

Si `count_usages` échoue, on ignore si la compétence est portée. La doctrine de
l'éditeur de roster s'applique — **le doute ferme la porte** — mais elle ne ferme
ici que ce qu'elle concerne :

| Mutation | Port indisponible |
|---|---|
| création | sans objet, le port n'est pas appelé |
| modification **sans** champ risqué | **passe** — le compte n'était pas nécessaire |
| modification **avec** champ risqué | `UsageUnavailable` |
| suppression | `UsageUnavailable` |

Traiter l'indisponibilité comme un zéro laisserait supprimer une compétence que
cent joueurs portent parce qu'une requête a échoué. Mais refuser une correction
de faute de frappe pour la même raison serait une sévérité gratuite : **la
question n'était pas posée.**

## Le cache n'apparaît pas dans les use cases

Toute écriture écrit en base **puis** rafraîchit la carte en mémoire (phase 3).
Ce second geste est un détail d'implémentation du dépôt.

```rust
// IReferenceWriteRepository — ce que le use case voit
async fn save_custom_skill(&self, space_id: &SpaceId, skill: &Skill, by: &CoachId)
    -> Result<(), RepositoryError>;
async fn delete_custom_skill(&self, uid: &CustomSkillUid) -> Result<(), RepositoryError>;
async fn find_custom_skill(&self, uid: &CustomSkillUid) -> Result<Option<CustomSkillRecord>, RepositoryError>;
```

`find_custom_skill` rend un **enregistrement**, pas un `Skill` : le use case a
besoin du `space_id` pour l'appartenance, et le `Skill` du corpus ne le porte
pas.

`save_custom_skill` sert la création **et** la modification — un `INSERT … ON
CONFLICT (uid) DO UPDATE`. Deux méthodes obligeraient le dépôt à distinguer deux
cas que le use case a déjà distingués.

**Le use case n'a pas à savoir qu'un cache existe.**

## Les erreurs

```rust
pub enum CustomSkillError {
    Forbidden,
    NotFound,
    /// Le compte est dans l'erreur : « 3 joueurs la portent », pas « impossible ».
    InUse { holders: u32 },
    /// Le champ est dans l'erreur : l'écran nomme ce qui s'est fermé.
    ImmutableFieldChanged { field: &'static str },
    NameTaken { name: String },
    Invalid(DomainError),
    UsageUnavailable(String),
    Repository(String),
}
```

`InUse` et `ImmutableFieldChanged` sont **deux erreurs et non une**, alors que
les deux viennent du même compte : la première dit « on ne peut pas la
supprimer », la seconde « on ne peut plus changer ce champ ». Les fondre
obligerait le contrôleur à deviner le message d'après la route empruntée.

## Ce que les use cases ne font pas

- **Aucune transaction.** Une écriture par mutation ; le rafraîchissement du
  cache ne participe à aucune transaction Postgres et ne le peut pas.
- **Aucune modification d'un joueur qui porte la compétence.** Renommer une
  compétence ne réécrit rien dans `players_proj` — le nom y est résolu par uid à
  l'affichage, et c'est ce qui rend le renommage sûr.
- **Aucun retrait de la compétence d'un roster.** Il ne peut pas s'en présenter
  (voir la suppression).
- **Aucune instrumentation à écrire à la main** : `#[tracing::instrument(skip_all,
  fields(cmd = ?cmd))]` sur les trois, comme l'axe 11 l'exige. La suppression, qui
  prend des identifiants nus, nomme ses champs : `fields(uid = ?uid, space_id =
  ?space_id)`.

## Règles métier à préciser

### 1 · Deux compétences peuvent-elles porter le même nom ?

Aucune règle de la phase 1 ne le dit. Je pose que **non, dans la liste que le
coach voit** — c'est-à-dire corpus **et** espace confondus, puisque
`list_skills_for_space` les sert ensemble et que le sélecteur les mélange.

Deux « Esquive » dans le même sélecteur, l'une à 20 kPo et l'autre à 30, est un
piège pour le coach qui choisit et une plainte pour l'organisateur qui devra
expliquer. Comparaison sur le nom **rogné et insensible à la casse** — sinon
« Esquive » et « esquive » cohabiteraient et le piège resterait entier.

Le coût est nul : la liste fusionnée est déjà ce que le use case peut demander.

### 2 · Un joueur licencié compte-t-il comme porteur ?

`count_usages` interroge `players_proj`. **Un joueur mort, licencié ou retiré y
figure-t-il encore ?** S'il y figure, il bloque la suppression ; sinon, elle
passe alors que sa fiche affiche toujours la compétence.

Je pose que **oui, il compte** — sa fiche reste consultable, et un uid qui n'y
résout plus rien y afficherait un vide. Bloquer une suppression de trop coûte un
libellé qui traîne ; l'inverse coûte une fiche muette, qui est exactement le
silence de la carte 438.

**À vérifier au moment d'écrire la requête**, pas à supposer : c'est la
projection qui a le dernier mot sur ce qu'elle garde.
