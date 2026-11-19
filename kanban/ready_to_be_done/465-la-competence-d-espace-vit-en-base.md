# La compétence d'espace vit en base

**Épic :** E10 — Référentiels éditables · **Ordre :** 3 · **Dépend de :** 441
**Conception :** `docs/specs/competences-personnalisees/page-de-gestion/`
(`03-back.md`, `07-integration.md`)

## Objectif

Une compétence saisie se range en base, revient au catalogue sans redémarrage, et
se résout comme n'importe laquelle du règlement. Aucun écran.

## La table

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

**En colonnes, là où les rosters sont en JSONB.** Un roster est un document
imbriqué qu'on lit en bloc ; une compétence est six champs plats, et la catégorie
se filtre — un `WHERE` sur du JSONB serait une gêne pour rien.

**`updated_at` existe ici**, contrairement au grand livre de trésorerie : une
compétence se modifie.

**Aucun rattrapage** : la table naît vide.

## Trois méthodes de dépôt

```rust
// s'ajoutent à IReferenceWriteRepository, créé par la carte 441
async fn save_custom_skill(&self, space_id: &SpaceId, skill: &Skill, by: &CoachId)
    -> Result<(), RepositoryError>;
async fn delete_custom_skill(&self, uid: &CustomSkillUid) -> Result<(), RepositoryError>;
async fn find_custom_skill(&self, uid: &CustomSkillUid)
    -> Result<Option<CustomSkillRecord>, RepositoryError>;
```

`save_custom_skill` est un `INSERT … ON CONFLICT (uid) DO UPDATE` : il sert la
création **et** la modification. Deux méthodes obligeraient le dépôt à distinguer
deux cas que le use case a déjà distingués.

`find_custom_skill` rend un **enregistrement** et non un `Skill` : le use case a
besoin du `space_id` pour l'appartenance, et le `Skill` du corpus ne le porte pas.

## Le cache — deux cartes

```rust
custom_skills:          RwLock<HashMap<String, Skill>>,       // uid → compétence
custom_skills_by_space: RwLock<HashMap<String, Vec<String>>>, // espace → uids
```

**À côté du corpus, pas dedans** : le corpus est immuable, le mettre derrière un
verrou ferait payer un lock à chaque lecture d'une donnée qui ne change jamais.

**Deux cartes et non une** : sans la seconde, lister les compétences d'un espace
demanderait de parcourir toute la première à chaque ouverture du sélecteur.

L'écriture écrit en base **puis** rafraîchit les deux. Un échec de
rafraîchissement part en `ERROR` — la base fait foi, un redémarrage remet tout
d'aplomb.

> **Le précédent à ne pas répéter** : carte 362, « le bundle CSS est gelé au
> démarrage » — un cache que rien ne rafraîchit et dont l'obsolescence ne se
> signale pas.

## L'aiguillage par préfixe

```rust
fn find_skill_by_uid(&self, uid: &str) -> Option<Skill> {   // était Option<&Skill>
    if uid.starts_with("CUSTOM_") {
        self.custom_skills.read().ok()?.get(uid).cloned()
    } else {
        self.skills.iter().find(|s| s.uid == uid).cloned()
    }
}
```

**Exhaustif, jamais un repli** : un uid personnalisé introuvable rend `None`, il
ne retombe pas sur le corpus.

**Il ne porte pas d'espace, et n'en a pas besoin** : résoudre une compétence par
identifiant doit marcher partout — un joueur vu depuis un autre espace affiche
ses compétences. **Ce qui se garde par l'espace, c'est le choix, pas la lecture.**

### Sept sites d'appel, tous en retirant un `&`

```
consistency.rs:82                     skill_catalog_adapter.rs:46
reference_data_adapter.rs:43, :79, :107, :118
roster_catalog_adapter.rs:23
```

Aucun ne conserve la référence au-delà de l'expression.

## `list_skills_for_space`

```rust
fn list_skills(&self) -> &[Skill];                              // le corpus, inchangé
fn list_skills_for_space(&self, space_id: &str) -> Vec<Skill>;  // corpus + espace
```

**Deux méthodes et non une élargie** : les appelants ne posent pas la même
question. Le corpus seul sert la vérification de cohérence ; la liste fusionnée
sert tout ce qu'un coach choisit.

## La cohérence au démarrage

`check_consistency` doit compter les compétences personnalisées — sinon un roster
d'espace qui en pose une ferait échouer la vérification.

**Mais l'échec ne doit pas empêcher le démarrage** pour une donnée saisie :
`WARN` et écartement du cache, jamais un panic. Même règle que pour les rosters.

## Tests

| Test | Ce qu'il prouve |
|---|---|
| `une_competence_enregistree_se_relit_par_son_uid` | l'aiguillage |
| `un_uid_custom_inconnu_ne_retombe_pas_sur_le_corpus` | le repli exclu |
| `list_skills_ne_rend_que_le_corpus` | la séparation des deux méthodes |
| `list_skills_for_space_rend_le_corpus_et_l_espace` | la fusion |
| `list_skills_for_space_ignore_les_autres_espaces` | le cloisonnement |
| `un_enregistrement_rafraichit_le_cache_sans_redemarrage` | **la carte 362** |
| `une_suppression_retire_l_uid_des_deux_cartes` | pas de fuite |
| `save_deux_fois_le_meme_uid_met_a_jour` | le `ON CONFLICT` |

Tests d'intégration sur une vraie `PgPool` — pas de mock sqlx.

## Checklist

- [ ] La migration
- [ ] Les trois méthodes sur `IReferenceWriteRepository`
- [ ] Les deux cartes de cache, rafraîchies à l'écriture
- [ ] `find_skill_by_uid` en `Option<Skill>`, **les sept sites adaptés**
- [ ] `list_skills_for_space`
- [ ] `check_consistency` élargi, en `WARN` et non en panic
- [ ] Les huit tests
- [ ] `make lint && make test && make check-arch`
