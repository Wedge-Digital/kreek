# Page de gestion · Phase 3 : architecture back

**Phase 2** : `02-front.md`

## Une correction de comptage, d'abord

La phase 2 annonçait « seize sites lisent le catalogue ». C'était le nombre de
**fichiers qui mentionnent une compétence**, ports compris. Le compte réel des
appels directs au dépôt de références est de **dix** :

| Méthode | Sites |
|---|---|
| `list_skills()` | 3 — `skill_picker.rs:158`, `team_created_listener.rs:288`, `skill_catalog_adapter.rs:52` |
| `find_skill_by_uid()` | 7 — dont 4 dans `reference_data_adapter.rs` |

C'est moins que redouté, mais **la difficulté n'est pas le nombre**.

## Le vrai obstacle — `list_skills()` ne sait pas de quel espace on parle

Pour les rosters personnalisés (carte 441), l'affaire était simple :
`find_team_by_uid` résout **par identifiant**, et le préfixe `CUSTOM_` décide où
regarder. `list_teams()` n'avait besoin que du corpus — ses deux appelants
étaient une vérification de cohérence et une migration.

Ici, c'est l'inverse. **`list_skills()` sert le sélecteur de compétences**, qui
doit montrer celles de l'espace :

```rust
// skill_picker.rs:157 — le sélecteur qu'un coach ouvre pour dépenser ses SPP
let skills: Vec<SkillRowVm> = repo.list_skills()
    .iter()
    .filter(|s| accessible.contains(s.category.as_str()))
```

Et sa route ne porte **aucun espace** :

```
/references/roster-lines/skill-picker
```

Pas de `{space_id}`, pas de `Query` qui en porte un. Le sélecteur ne sait pas où
il est.

### Ce qu'il faut donc faire, et que la 441 n'imposait pas

```rust
// avant
fn list_skills(&self) -> &[Skill];

// après
fn list_skills(&self) -> &[Skill];                       // le corpus, inchangé
fn list_skills_for_space(&self, space_id: &str) -> Vec<Skill>;   // corpus + espace
```

**Deux méthodes et non une élargie.** Les trois appelants ne posent pas la même
question :

| Appelant | Question | Méthode |
|---|---|---|
| `skill_picker.rs` | « que peut apprendre ce joueur ? » | `for_space` |
| `skill_catalog_adapter.rs` | « quel est le catalogue complet ? » | `for_space` |
| `team_created_listener.rs:288` | « quelles compétences de base pour ce poste ? » | **à vérifier** |

Le troisième mérite examen : s'il résout les compétences de départ d'un roster,
et qu'un roster personnalisé peut poser une compétence personnalisée, alors il
lui faut l'espace aussi.

### La route du sélecteur gagne son espace

```
/app/{space_id}/references/roster-lines/skill-picker
```

Son unique appelant — `finalize-team.html:81` — connaît son `space_id`. Le
changement est mécanique côté gabarit ; côté route, il fait entrer le sélecteur
sous `space_scope`, ce qui est **un gain** : aujourd'hui il sert le catalogue à
qui le demande, sans notion d'espace.

## La signature qui change — comme pour les rosters

```rust
fn find_skill_by_uid(&self, uid: &str) -> Option<Skill>;   // était Option<&Skill>
```

Même raison que la carte 441 : une compétence en base ne peut pas être rendue
par référence depuis un `RwLock`. Sept sites d'appel, tous en retirant un `&`.

**L'aiguillage est exhaustif**, jamais un repli :

```rust
if uid.starts_with(CUSTOM_PREFIX) {
    self.custom_skills.read().ok()?.get(uid).cloned()
} else {
    self.skills.iter().find(|s| s.uid == uid).cloned()
}
```

Un uid personnalisé introuvable rend `None` — il ne retombe pas sur le corpus.

**Mais une différence avec les rosters** : `find_skill_by_uid` ne porte pas
d'espace, et n'en a **pas besoin**. Résoudre une compétence par identifiant doit
marcher partout — un joueur vu depuis un autre espace affiche ses compétences.
C'est le même principe que la résolution d'un roster (carte 441, décision de
portée) : **ce qui se garde par l'espace, c'est le choix, pas la lecture.**

## Persistance

```sql
CREATE TABLE references__custom_skills (
    uid         TEXT PRIMARY KEY,          -- CUSTOM_<sulid>
    space_id    TEXT NOT NULL,
    name        TEXT NOT NULL,
    category    TEXT NOT NULL,
    skill_type  TEXT NOT NULL,             -- « Standard » | « Élite »
    activation  TEXT NOT NULL,             -- « Active » | « Passive »
    description TEXT NOT NULL,
    created_by  TEXT NOT NULL,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at  TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX ON references__custom_skills (space_id);
```

**En colonnes et non en JSONB**, contrairement aux rosters personnalisés. Un
roster est un document imbriqué qu'on lit en bloc ; une compétence est **six
champs plats**, et la catégorie se filtre — `WHERE category = $1` sur du JSONB
serait une gêne pour rien.

**`updated_at` existe ici**, alors que le grand livre de trésorerie n'en avait
pas : une compétence **se modifie** (règle 6), contrairement à une ligne de
relevé.

## Le cache, comme pour les rosters

```rust
pub struct InMemoryReferenceRepository {
    …                                                // les treize existantes
    custom_teams:  RwLock<HashMap<String, Team>>,    // carte 441
    custom_skills: RwLock<HashMap<String, Skill>>,   // ← celle-ci
}
```

**À côté du corpus, pas dedans.** Le corpus est immuable ; le mettre derrière un
verrou ferait payer un lock à chaque lecture d'une donnée qui ne change jamais.

Et une seconde carte indexée par espace, pour `list_skills_for_space` :

```rust
custom_skills_by_space: RwLock<HashMap<String, Vec<String>>>,   // space → uids
```

Sans elle, lister les compétences d'un espace demanderait de parcourir toute la
carte à chaque ouverture du sélecteur.

**Le rafraîchissement fait partie de l'écriture** : base d'abord, cache ensuite,
et un échec part en `ERROR` (le précédent de la carte 362, le bundle CSS gelé).

## « Cette compétence est-elle employée ? »

C'est ce qui décide du verrou (règles 5 et 6). Contrairement aux rosters — où la
question n'avait aucune réponse et exigeait une colonne neuve (carte 442) —
**celle-ci est directement interrogeable** :

```sql
SELECT count(*) FROM players_proj
WHERE  acquired_skills @> jsonb_build_array(jsonb_build_object('skill_id', $1))
```

`players_proj.acquired_skills` est un JSONB qui porte `skill_id`.

**Mais un joueur n'est pas le seul porteur.** Une compétence peut aussi être une
**compétence de base d'un poste**, dans un roster personnalisé (épic E10) — et
là, aucune requête ne la trouve : elle vit dans le JSONB `definition` d'un
roster.

Deux comptes, donc, et le port doit les additionner :

```rust
#[async_trait]
pub trait ISkillUsagePort: Send + Sync {
    /// Combien de joueurs l'ont acquise, plus combien de postes la posent
    /// en compétence de base. Zéro autorise la suppression.
    async fn count_usages(&self, skill_uid: &str) -> Result<u32, String>;
}
```

**Oublier le second compte laisserait supprimer une compétence qu'un roster
pose**, et le poste afficherait alors un uid mort — exactement le défaut de la
carte 438, le `filter_map` muet.

## Les mutations

```
create_custom_skill_use_case   — tous les champs
update_custom_skill_use_case   — nom et description seuls si employée
delete_custom_skill_use_case   — refusée si employée
```

### Le verrou partiel vit dans le use case, pas dans le domaine

```rust
if usage_count > 0 && (cmd.category != current.category || cmd.skill_type != current.skill_type) {
    return Err(ImmutableFieldChanged { field: … });
}
```

**Il ne peut pas être dans le domaine** : il compare l'état persisté au compte
d'usage, qui vient d'un port. C'est la même ligne que pour `with_inducements_from`
sur les tiers de compétition (épic E14) — sauf qu'ici la condition dépend d'une
donnée extérieure, donc elle reste applicative.

**Un écart est un refus, pas une correction silencieuse** : accepter la valeur
reçue déplacerait rétroactivement le coût d'un achat fait.

### L'accès

Admin d'espace. `references` déclarera son port — le même que celui de l'épic
E10 (`IReferencesSpaceAdminPort`), **s'il est déjà posé**, sinon il le pose.

## La cohérence au démarrage

`check_consistency` vérifie que chaque roster ne référence que des compétences
existantes. Une compétence personnalisée doit y compter — sinon un roster
d'espace qui en pose une ferait échouer la vérification.

**Mais l'échec ne doit pas empêcher le démarrage** pour une compétence saisie :
`WARN` et écartement du cache, jamais un panic. C'est déjà la règle posée pour
les rosters personnalisés (carte 441).

## Ce que le back ne fait pas

- **Aucune migration de données** : la table naît vide.
- **Aucun événement** : personne n'a à réagir à une compétence qui apparaît —
  elle se résout par uid au moment d'en avoir besoin.
- **Aucun changement au chargement du corpus.**

Une exception possible à ce deuxième point : **la suppression**. Si une
compétence supprimée était posée par un roster personnalisé, il faudrait l'en
retirer — comme `CustomRosterDeleted` nettoie les tiers de compétition. Mais la
règle 5 l'interdit déjà : **une compétence employée n'est pas supprimable**, et
un roster qui la pose compte comme un usage. Le cas ne peut donc pas se produire.

## Livraison conjointe avec l'épic E10

**Les rosters personnalisés et les compétences personnalisées partent
ensemble.** Ce n'est pas une commodité de planning : les deux se tiennent par
les deux bouts.

### Ce que ça règle

La seule règle métier restée ouverte — *un roster personnalisé qui pose une
compétence personnalisée compte-t-il comme un usage bloquant ?* — n'a plus de
variante. **Oui, et dès le premier jour** : `references__custom_rosters` (carte
441) existera quand `ISkillUsagePort` l'interrogera. Le second compte n'est pas
un repli à zéro qu'on rebranchera plus tard, c'est la moitié de la réponse.

Cela impose **un seul ordre** : la carte qui pose `ISkillUsagePort` vient après
la 441, qui crée la table. C'est la seule contrainte de séquence entre les deux
séries.

### Ce que ça découvre — un troisième appelant

L'éditeur de roster (carte 446) sert un sélecteur de compétences pour les
compétences de base d'un poste :

> 146 compétences, 38 mots-clefs, les catégories, le staff, les règles spéciales.

**Ce sélecteur-là aussi doit montrer les compétences de l'espace.** Sans quoi on
livrerait le jour même deux fonctionnalités qui s'ignorent : un espace pourrait
créer une compétence et un roster, sans pouvoir poser l'une dans l'autre — et
c'est pourtant l'emploi le plus évident des deux ensemble.

`list_skills_for_space` a donc **trois appelants et non deux** :

| Appelant | Statut |
|---|---|
| `skill_picker.rs` | existant, sa route gagne un `space_id` |
| `skill_catalog_adapter.rs` | existant |
| l'éditeur de roster (carte 446) | **à naître, et à écrire d'emblée avec l'espace** |

Le troisième est le moins cher des trois — il n'existe pas encore, donc rien à
migrer. Mais c'est celui qu'on oublierait, puisqu'il ne figure dans aucun
inventaire de l'existant.

### Ce que ça oblige à corriger ailleurs

L'épic E10 exclut aujourd'hui explicitement ce chantier :

> **Les autres référentiels** — compétences, coups de pouce, star players — qui
> restent en lecture seule.

Cette phrase devient fausse. Elle sera reprise en phase 8, avec l'entrée des
cartes de compétences dans l'épic et le « Terminé quand » qui doit désormais
mentionner les deux — sans quoi l'épic se clôturerait sur la moitié de ce
qu'elle livre.
