# Éditeur de roster · Phase 3 : architecture back

**Phase 2** : `02-front.md`

## Le mur, et il faut le regarder avant tout le reste

Un roster se résout aujourd'hui ainsi :

```rust
// references/domain/port.rs:22
fn find_team_by_uid(&self, uid: &str) -> Option<&Team>;
```

**Synchrone, et elle rend une référence empruntée** au corpus tenu en mémoire.
Un roster qui vit en base ne peut pas être rendu par cette signature : il faut
le charger, donc `async`, et le posséder, donc `Team` et non `&Team`.

Et la contamination va loin. Les huit appelants sont synchrones, et les ports
qu'ils servent le sont aussi :

```rust
fn find_roster_definition(&self, roster_uid: &str) -> Option<RosterDefinition>;  // team_creation
fn find_catalog(&self, roster_id: &str) -> Option<RosterCatalogDto>;             // teams
fn journeyman_type_for_roster(&self, roster_id: &str) -> JourneymanTypeDto;      // teams
```

Rendre le port asynchrone, c'est rendre asynchrones ces trois-là, puis leurs
appelants — dont `resolve_team_value`, `roster_service::load_roster`, et le
rendu de plusieurs écrans. **Pour lire un roster que personne ne modifie
pendant la requête.**

## La voie retenue : le dépôt sert deux sources, et reste synchrone

`InMemoryReferenceRepository` garde ses treize collections et en gagne une
quatorzième : les rosters d'espace, chargés depuis la base **au démarrage**, et
rafraîchis **à chaque écriture**.

```rust
pub struct InMemoryReferenceRepository {
    …                                        // les treize existantes, inchangées
    custom_teams: RwLock<HashMap<String, Team>>,   // clef = uid, préfixé CUSTOM_
}
```

`RwLock` et non `Mutex` : les lectures sont massives et concurrentes, les
écritures rarissimes — un roster créé de temps à autre.

### Deux collections, une seule porte — et non une collection fusionnée

Les rosters personnalisés vivent **à côté** du corpus, pas dedans. Trois raisons,
et la troisième est décisive :

- le corpus est **immuable** et chargé une fois ; le mettre derrière un verrou
  ferait payer une prise de lock à chaque lecture d'un roster qui ne change
  jamais ;
- les deux n'ont pas la même durée de vie ni la même source de vérité — l'un
  vient d'un fichier, l'autre d'une table ;
- `list_teams()` rend `&[Team]`, **une tranche empruntée**. Une collection unique
  derrière un `RwLock` ne peut pas produire cette signature ; deux collections la
  préservent pour le corpus, qui est le seul à en avoir besoin.

C'est le préfixe qui fait l'unité : une seule porte d'entrée,
`find_team_by_uid`, qui sait de quel côté regarder.

### `list_teams()` reste le corpus, et c'est délibéré

Ses deux appelants ne veulent que lui :

| Appelant | Ce qu'il fait |
|---|---|
| `consistency.rs:47` | vérifie le corpus au démarrage |
| `m002_recalcul_valeurs_equipe.rs:85` | migration de données au démarrage |

Aucun des deux n'a affaire aux rosters d'espace, et les leur servir changerait
leur sens sans qu'ils le demandent. La liste des rosters d'un espace passe par
une **méthode distincte** — `list_teams_for_space(space_id)` — parce que c'est
une autre question.

### Ce que ça préserve

**Aucune signature ne change.** `find_team_by_uid` reste synchrone. Les huit
appelants, les trois ports consommateurs et tous leurs appelants restent
inchangés. C'est la raison du choix, et elle vaut à elle seule.

### Ce que ça coûte, dit franchement

`find_team_by_uid` rend `Option<&Team>`, une **référence** — impossible à
produire depuis un `RwLock` sans garder le verrou. La méthode doit donc
devenir :

```rust
fn find_team_by_uid(&self, uid: &str) -> Option<Team>;   // par valeur
```

**Un `clone` par lecture.** Un `Team` porte une poignée de `String` et son
`Vec<PlayerPosition>` — quelques kilooctets. Les huit sites d'appel s'adaptent
en retirant un `&` ; aucun ne conserve la référence au-delà de l'expression.

C'est le seul changement de signature de toute la fonctionnalité, et il est
mécanique. Le comparer au ripple asynchrone tranche tout seul.

## Le préfixe `CUSTOM_` — ce qu'il apporte et ce qu'il n'apporte pas

```rust
fn find_team_by_uid(&self, uid: &str) -> Option<Team> {
    if uid.starts_with(CUSTOM_PREFIX) {
        self.custom_teams.read().ok()?.get(uid).cloned()
    } else {
        self.teams.iter().find(|t| t.uid == uid).cloned()
    }
}
```

**Exhaustif, jamais « essayer l'un puis l'autre ».** Le repli réintroduirait
exactement la double interrogation que le préfixe supprime, et masquerait une
erreur : un uid personnalisé introuvable doit rendre `None`, pas aller chercher
dans le corpus.

**Le préfixe est engendré, jamais saisi.** L'uid se fabrique côté serveur —
`CUSTOM_` suivi d'un identifiant du service d'identifiants — et n'apparaît
nulle part dans le formulaire. Un préfixe qu'un humain pourrait taper serait un
type encodé dans une chaîne, à la merci d'une faute de frappe.

**À vérifier avant de livrer** : aucun uid du corpus de production ne commence
par `CUSTOM_`. Le corpus de démonstration est propre ; celui de production vit
hors du dépôt.

```bash
grep -c '"uid": "CUSTOM_' "$REFERENCES__DIR"/teams_*.json
```

## Persistance

```sql
CREATE TABLE references__custom_rosters (
    uid         TEXT PRIMARY KEY,          -- CUSTOM_<sulid>
    space_id    TEXT NOT NULL,
    definition  JSONB NOT NULL,            -- le Team complet, postes compris
    created_by  TEXT NOT NULL,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at  TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX ON references__custom_rosters (space_id);
```

**Le roster entier en JSONB, et non un schéma éclaté.** Un `Team` est une
donnée de référence : on le lit en bloc, on l'écrit en bloc, et rien ne le
requête par poste. Éclater les postes en table fille donnerait cinq jointures
pour reconstruire ce que `serde` rend en une ligne. Et le JSONB **est déjà** la
forme du corpus : la sérialisation existe, il n'y a rien à écrire.

`space_id` en colonne et non dans le JSONB : c'est la seule chose qu'on filtre.

## La portée, et où elle se fait respecter

Un roster d'espace n'est visible que dans son espace. **Le dépôt de références
ne le sait pas** — il rend un `Team` par uid, sans notion d'espace.

Le filtre vit donc à deux endroits, et il faut les nommer :

| Où | Ce qui est filtré |
|---|---|
| **La liste** — `list_teams_for_space(space_id)` | ce qu'un coach peut choisir |
| **La résolution** — `find_team_by_uid(uid)` | **rien** |

**La résolution ne filtre pas, et c'est voulu.** Une équipe déjà créée doit
pouvoir résoudre son roster où qu'on la regarde — dans un classement, un
rapport de match, une page publique. Filtrer par espace à la résolution
casserait l'affichage d'une équipe vue depuis ailleurs.

Ce qui se garde par l'espace, c'est **le choix**, pas la lecture. Et le choix
passe par la liste, qui, elle, filtre.

## Qui peut lire quoi

| Question | Réponse |
|---|---|
| Qui voit la liste des rosters de l'espace ? | tout membre — c'est ce qu'il pourra choisir |
| Qui crée, modifie, supprime ? | **admin d'espace seulement** |

L'administration d'un espace a déjà son contrôle
(`ICompetitionSpaceMemberPort::find_member_profile` → `SpaceProfile::SpaceAdmin`).
`references` n'y a pas accès et déclarera son propre port.

## « Ce roster est-il utilisé ? » — la vraie difficulté

C'est la question qui commande le verrou, et **elle n'a pas de réponse facile**.

**Aucune table ne porte le uid du roster d'une équipe.** `team_proj` n'a qu'un
`roster_name` d'affichage. L'information vit dans la charge utile de
`TeamCreated`, au fond de l'event store de `teams` :

```sql
select payload->>'roster_id', count(*) from team_event_store
where event_type='TeamCreated' group by 1;
--  DEMO_GRANIT | 1855
```

Or `references` n'a pas le droit d'interroger l'event store d'un autre BC.

### Un port, et son adapter

```rust
// references/ports.rs — le fichier n'existe pas encore
#[async_trait]
pub trait IRosterUsagePort: Send + Sync {
    /// Combien d'équipes jouent ce roster. Zéro autorise la modification.
    async fn count_teams_using(&self, roster_uid: &str) -> Result<u32, String>;
}
```

`src/infrastructure/references/roster_usage_adapter.rs` — **le dossier n'existe
pas non plus** : `references` n'a jamais eu besoin de sortir de lui-même.

### La colonne qui manque — `team_proj.roster_id`

Interroger `team_event_store` par `payload->>'roster_id'` fonctionnerait, mais
ce serait **balayer un flux d'événements pour répondre à une question d'état**,
sans index, ce que le projet range du côté des projections.

**La colonne est donc ajoutée** (tranché). L'adapter compte alors une ligne
indexée :

```sql
SELECT count(*) FROM team_proj WHERE roster_id = $1
```

Trois gestes, et un seul est de cette fonctionnalité :

#### 1. Le schéma et le rattrapage — une seule migration SQL

```sql
-- migrations/<date>_team_proj_roster_id.sql
ALTER TABLE team_proj ADD COLUMN roster_id TEXT;

-- Rattrapage des équipes déjà présentes : le uid vit dans la charge utile
-- de TeamCreated, et n'a jamais été projeté.
UPDATE team_proj p
SET    roster_id = e.payload->>'roster_id'
FROM   team_event_store e
WHERE  e.team_id = p.team_id
  AND  e.event_type = 'TeamCreated'
  AND  p.roster_id IS NULL;

CREATE INDEX ON team_proj (roster_id);
```

**Du SQL, et non le registre de migrations Rust.** Ce dernier
(`infrastructure/data_migrations/`) existe pour les rattrapages qui ont besoin
du **corpus** — `m001_bonus_elite` et `m002_recalcul_valeurs_equipe` recalculent
des valeurs à partir des prix de référence, et ne peuvent donc être écrits qu'en
Rust, au démarrage. Ici, la donnée est déjà en base : du SQL suffit, et il
s'exécute avec les autres migrations.

Le précédent est là : `20260824000001_..._notifications_off_for_existing.sql`
corrige des données existantes en SQL pur.

**L'index après le rattrapage**, pas avant : le construire sur une colonne
qu'on est en train de remplir coûte deux fois.

**La colonne reste nullable.** Une équipe dont l'événement `TeamCreated` serait
introuvable garde `NULL` plutôt que d'échouer la migration — et un `NULL` ne
compte dans aucun `WHERE roster_id = $1`, donc il ne verrouille rien à tort.
À contrôler après passage :

```sql
SELECT count(*) FROM team_proj WHERE roster_id IS NULL;   -- doit valoir 0
```

#### 2. Le projecteur écrit la colonne désormais

`team_repository.rs:32` destructure `TeamCreated` **sans prendre `roster_id`**,
alors que l'agrégat le porte (`team.rs:58`). Il suffit de l'ajouter à la
destructuration, à l'`INSERT` et à l'`ON CONFLICT DO UPDATE`.

C'est le geste qui empêche la colonne de se re-vider : sans lui, le rattrapage
serait juste au jour de la livraison et faux le lendemain.

#### 3. L'adapter compte

`infrastructure/references/roster_usage_adapter.rs` lit `team_proj`. Il est le
seul de `references` à connaître `teams`.

### Le verrou doit être re-vérifié au POST

L'écran décide d'afficher « Modifier » sur un compteur à zéro. Entre l'affichage
et l'enregistrement, une équipe peut naître. **Le use case recompte**, et refuse
si le compte a changé — l'écran avertit, le serveur tranche.

## Supprimer : le verrou des équipes, puis la propagation

« Zéro équipe » autorise la suppression. **Ça ne suffit pas**, parce qu'un uid de
roster est recopié à deux autres endroits, et qu'aucun des deux n'est une équipe.

| Où | Ce qui s'y trouve |
|---|---|
| `competition_seasons.rules → tiers[].rosters` | les rosters qu'une saison autorise |
| `team_drafts.creation_rules → tiers[].rosters` | **une copie** de cette liste, figée à la création du brouillon |

Vérifié en base : un brouillon ne choisit pas *un* roster, il **recopie la liste
des rosters autorisés** de son tier.

```json
{"tiers": [{"name": "Tier 1", "budget": 1060,
            "rosters": ["DEMO_GRANIT", "DEMO_LANTERNE", "DEMO_ZEPHYR"], …}]}
```

### Ce qu'une suppression non gardée produirait

Un uid qui ne résout plus, dans un tier de compétition. Et c'est **exactement**
le défaut qu'on a diagnostiqué en production sur un roster Slann :

```rust
// team_creation/io/web/builders.rs:104
tier.rosters.iter().filter_map(move |uid| {
    ref_data.find_roster_definition(uid).map(|def| …)
})
```

Le `filter_map` **laisse tomber en silence** tout uid introuvable. Le roster
disparaît du sélecteur de création d'équipe sans une ligne de journal, et le
ligueur cherche du côté du corpus.

### La règle retenue : supprimer, puis propager

**Un roster est supprimable dès qu'aucune équipe ne le joue.** Les tiers qui le
citent ne bloquent pas : ils sont **mis à jour après coup**, par un app event.

```
Suppression (references)
    │  DomainEvent  CustomRosterDeleted
    ▼
Publisher (io) ──► AppEvent  ReferencesAppEvent::CustomRosterDeleted { roster_uid }
    │
    ▼
Listener (competitions) ──► retire l'uid des tiers de toutes les saisons
```

C'est la doctrine du `CLAUDE.md` appliquée à la lettre : **propagation d'un
effet → app event**, jamais une consultation bloquante. Et c'est plus souple —
le jour où un troisième BC recopiera des uid de roster, il écoutera le même
événement sans que `references` ait à le connaître.

Les **brouillons ne verrouillent rien** et n'ont pas non plus à être nettoyés :
leur liste est une copie figée, un uid mort y est sans effet, et le brouillon se
refait.

### Ce que l'asynchronisme coûte, et ce qu'il faut faire avec

Entre la suppression et le passage du listener, **un tier cite encore un uid
mort**. La fenêtre est courte — le bus est en mémoire — mais elle existe, et si
le listener échoue, elle ne se referme jamais.

Or c'est précisément là que `builders.rs:104` fait des dégâts :

```rust
tier.rosters.iter().filter_map(move |uid| {
    ref_data.find_roster_definition(uid).map(|def| …)
})
```

**Le `filter_map` laisse tomber en silence** tout uid introuvable. Le roster
disparaît du sélecteur de création d'équipe sans une ligne de journal — c'est le
défaut diagnostiqué en production sur un roster Slann, et il existe **déjà**,
indépendamment de cette fonctionnalité.

Deux conséquences :

1. **Ce `filter_map` doit journaliser**, et c'est une carte à part — le défaut
   ne vient pas d'ici et ne doit pas être réparé en douce sous une carte de
   roster personnalisé.
2. **Le listener journalise son passage** : combien de saisons touchées, combien
   de tiers modifiés. Un listener silencieux qui échoue laisse une incohérence
   que rien ne raconte.

Avec le premier point réglé, la fenêtre asynchrone devient bénigne : un uid mort
se voit dans le journal au lieu de s'évaporer.

## Le rafraîchissement du cache

Toute écriture — création, modification, suppression — fait deux choses **dans
cet ordre** :

```
1. écrire en base
2. rafraîchir custom_teams pour ce uid
```

**La base d'abord.** Si le rafraîchissement échoue, la base fait foi et un
redémarrage remet tout d'aplomb. L'inverse laisserait un roster en mémoire que
rien ne porte.

**Ce n'est pas une transaction et ça ne peut pas l'être** : un `HashMap` en
mémoire ne participe à aucune transaction Postgres. La fenêtre est d'une
poignée de microsecondes, et le pire cas est un roster absent du sélecteur
jusqu'au prochain démarrage.

> **Le précédent à ne pas répéter.** La carte 362 — « le bundle CSS est gelé au
> démarrage » — décrit exactement le défaut inverse : un cache chargé au
> démarrage que **rien ne rafraîchit**, et dont l'obsolescence ne se signale
> pas. Ici le rafraîchissement fait partie de l'écriture, et son échec se
> journalise en `ERROR`.

## La cohérence, qui ne doit pas devenir partielle

`load_from_dir` appelle `verifier_coherence_des_haines()`, et `check_consistency`
vérifie que chaque roster ne référence que des compétences, mots-clefs,
catégories, staff et règles spéciales **qui existent**.

Un roster personnalisé doit passer les mêmes contrôles — mais **à
l'enregistrement**, pas au démarrage : un refus au démarrage sur une donnée
saisie hier rendrait l'application inaccessible pour tout le monde.

| Quand | Quoi | En cas d'échec |
|---|---|---|
| À l'enregistrement | les cinq contrôles de `check_consistency` sur ce roster | `422`, l'écran dit ce qui manque |
| Au démarrage | les mêmes, sur les rosters chargés depuis la base | **journal `WARN`, roster écarté du cache** — jamais un panic |

Le second cas ne devrait jamais arriver : ce qui l'y met, c'est un corpus qui
change sous les pieds d'un roster enregistré — une compétence retirée du
fichier. Écarter le roster et le dire vaut mieux qu'un serveur mort.

## Ce que le back ne fait pas

- **Une migration**, et une seule : `team_proj` gagne `roster_id`, rattrapée en
  SQL depuis les charges utiles de `TeamCreated`. La table des rosters
  personnalisés, elle, naît vide.
- **Un seul événement**, et uniquement à la suppression :
  `CustomRosterDeleted`, converti en app event par le publisher de `references`
  — qui **n'existe pas encore**, ce BC n'ayant jamais rien publié. La création
  et la modification n'en émettent aucun : personne n'a à réagir à un roster qui
  apparaît ou change, puisque tout le monde le résout par uid au moment d'en
  avoir besoin.
- **Aucun changement au chargement du corpus.**

## Règles métier à préciser

1. **Une règle spéciale absente du corpus** reste refusée (phase 2). Confirmé
   par la mécanique : `resoudre_bareme` associe une règle spéciale à un barème
   de SPP par son nom en minuscules ; une règle inventée retomberait sur
   `normal` sans rien dire.

2. **Tranché** : `team_proj.roster_id` est ajoutée, rattrapée en SQL, et le
   projecteur l'écrit désormais. C'est un ajout au BC `teams` porté par cette
   fonctionnalité parce qu'elle en a besoin — mais il servira à quiconque voudra
   savoir quelles équipes jouent un roster, ce qu'aucune requête ne permet
   aujourd'hui.
