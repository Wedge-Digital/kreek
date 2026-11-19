# Le roster d'espace vit en base

**Épic :** E10 · **Ordre :** 1 · **Dépend de :** rien
**Conception :** `docs/specs/roster-personnalise/editeur-de-roster/03-back.md`

## Objectif

Qu'un roster puisse exister ailleurs que dans le corpus, et se résolve par le
même chemin que les autres. Aucun écran.

## Le mur, et pourquoi on ne le franchit pas

```rust
// references/domain/port.rs:22
fn find_team_by_uid(&self, uid: &str) -> Option<&Team>;
```

**Synchrone, et elle rend une référence empruntée.** Un roster en base imposerait
`async` et la possession — or les huit appelants sont synchrones, et les trois
ports qu'ils servent aussi (`find_roster_definition`, `find_catalog`,
`journeyman_type_for_roster`). Rendre le port asynchrone contaminerait
`resolve_team_value`, `roster_service::load_roster` et plusieurs rendus,
**pour lire un roster que personne ne modifie pendant la requête**.

## Conception

### Deux collections, une seule porte

```rust
pub struct InMemoryReferenceRepository {
    …                                              // les treize existantes
    custom_teams: RwLock<HashMap<String, Team>>,   // clef = uid préfixé
}
```

**À côté du corpus, pas dedans.** Le corpus est immuable : le mettre derrière un
verrou ferait payer un lock à chaque lecture d'une donnée qui ne change jamais.
Et `list_teams()` rend `&[Team]`, **une tranche empruntée** — une collection
unique sous `RwLock` ne peut pas produire cette signature.

`RwLock` et non `Mutex` : lectures massives, écritures rarissimes.

### L'aiguillage est exhaustif

```rust
fn find_team_by_uid(&self, uid: &str) -> Option<Team> {
    if uid.starts_with(CUSTOM_PREFIX) {
        self.custom_teams.read().ok()?.get(uid).cloned()
    } else {
        self.teams.iter().find(|t| t.uid == uid).cloned()
    }
}
```

**Jamais « essayer l'un puis l'autre ».** Le repli réintroduirait la double
interrogation que le préfixe supprime, et masquerait une erreur : un uid
personnalisé introuvable doit rendre `None`.

**Le préfixe est engendré, jamais saisi** — `CUSTOM_` suivi d'un identifiant du
service d'identifiants, invisible du formulaire.

### La signature qui change — huit sites

`Option<Team>` au lieu de `Option<&Team>`. Un `clone` de quelques kilooctets par
lecture, contre un ripple asynchrone dans la moitié de l'application.

```
league_selector.rs:49        special_rule_selector.rs:88   consistency.rs:65
reference_data_adapter.rs:20 ref_team_data_adapter.rs:96 et :112
journeyman_type_adapter.rs:29 roster_catalog_adapter.rs:61
```

Tous en retirant un `&` : aucun ne conserve la référence au-delà de
l'expression.

### `list_teams()` reste le corpus

Ses deux appelants — `consistency.rs:47` et `m002_recalcul_valeurs_equipe.rs:85`
— ne veulent que lui. Les rosters d'espace passent par
`list_for_space(space_id)`, parce que c'est une autre question.

### La table

```sql
CREATE TABLE references__custom_rosters (
    uid TEXT PRIMARY KEY, space_id TEXT NOT NULL, definition JSONB NOT NULL,
    created_by TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX ON references__custom_rosters (space_id);
```

Le `Team` entier en JSONB : lu en bloc, écrit en bloc, jamais requêté par poste,
**et c'est déjà la forme du corpus** — la sérialisation existe.

### L'écriture, dans un trait distinct

```rust
#[async_trait]
pub trait IReferenceWriteRepository: Send + Sync {
    async fn save_custom_roster(&self, space_id: &SpaceId, team: &Team, by: &CoachId) -> …;
    async fn delete_custom_roster(&self, uid: &RosterUid) -> …;
    async fn list_for_space(&self, space_id: &SpaceId) -> Result<Vec<Team>, …>;
}
```

**Distinct de `IReferenceRepository`**, qui est synchrone et en lecture seule.
Les fondre rendrait la lecture asynchrone — ce qu'on vient d'éviter.

### Le rafraîchissement fait partie de l'écriture

```
1. écrire en base
2. rafraîchir custom_teams pour ce uid
```

**La base d'abord** : si le rafraîchissement échoue, elle fait foi et un
redémarrage remet tout d'aplomb. L'échec part en `ERROR`.

Ce n'est **pas** une transaction et ça ne peut pas l'être : un `HashMap` ne
participe à aucune transaction Postgres. Le pire cas est un roster absent du
sélecteur jusqu'au prochain démarrage.

> **Le précédent à ne pas répéter** : carte 362, « le bundle CSS est gelé au
> démarrage » — un cache que rien ne rafraîchit, dont l'obsolescence ne se
> signale pas.

### Le chargement au démarrage

`load_from_dir` charge le corpus ; une seconde passe charge la table.

**Un roster de la base qui échoue les contrôles de cohérence est écarté avec un
`WARN`, jamais un panic.** Ce qui l'y mettrait : une compétence retirée du
corpus sous les pieds d'un roster enregistré. Écarter et le dire vaut mieux
qu'un serveur mort — c'est l'inverse du traitement du corpus, dont l'incohérence
**empêche** le démarrage.

## À vérifier avant de livrer

```bash
grep -c '"uid": "CUSTOM_' "$REFERENCES__DIR"/teams_*.json   # doit valoir 0
```

Le corpus de démonstration est propre ; celui de production vit hors du dépôt.

## Tests

| Test | Ce qu'il prouve |
|---|---|
| `un_uid_prefixe_va_chercher_en_memoire_custom` | l'aiguillage |
| `un_uid_du_corpus_ne_regarde_jamais_les_customs` | l'exhaustivité, pas de repli |
| `un_uid_custom_inconnu_rend_none` | et **ne** retombe **pas** sur le corpus |
| `save_puis_find_rend_le_roster_sans_redemarrage` | **le test du cache**, celui qui compte |
| `delete_puis_find_rend_none` | idem, dans l'autre sens |
| `list_teams_ne_rend_que_le_corpus` | les deux appelants gardent leur sens |
| `un_roster_incoherent_en_base_est_ecarte_avec_un_warn` | le démarrage survit |

## Checklist

- [ ] La table et sa migration
- [ ] `custom_teams`, l'aiguillage, le préfixe engendré
- [ ] `find_team_by_uid` par valeur — **les huit sites**
- [ ] `IReferenceWriteRepository` et son implémentation
- [ ] Le rafraîchissement, et son `ERROR` en cas d'échec
- [ ] Le chargement au démarrage, avec écartement des incohérents
- [ ] Le `grep` de vérification sur le corpus de production
- [ ] `make lint && make test && make check-arch`
