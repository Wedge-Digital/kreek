# Les trois use cases du roster personnalisé

**Épic :** E10 · **Ordre :** 2 · **Dépend de :** 440, 441, 442
**Conception :** `docs/specs/roster-personnalise/editeur-de-roster/05-use-cases.md`

## Objectif

Créer, modifier, supprimer. Et donner à `references` la couche applicative qu'il
n'a jamais eue.

## Ce que le BC gagne au passage

| Quoi | État |
|---|---|
| `references/use_cases/` | **n'existe pas** |
| `references/ports.rs` | posé par la carte 442 |
| Un bus interne dans son contexte | **n'existe pas** — le contexte tient en une ligne |

## Le geste commun

```
1. l'appelant est-il admin de cet espace ?        → Forbidden
2. le roster existe-t-il, et dans CET espace ?    → NotFound
3. combien d'équipes le jouent ?                  → InUse { teams }
```

**`NotFound` et non `Forbidden` pour un roster d'un autre espace.** Un
`Forbidden` confirmerait son existence à qui énumère — c'est la règle que
`space_scope` applique déjà : « `404` et non `403` pour une ressource étrangère ».

L'étape 3 ne concerne pas la création.

## 1 · Créer

```rust
pub async fn execute(cmd: CreateCustomRosterCommand, repo: &dyn IReferenceWriteRepository,
                     refs: &dyn IReferenceRepository, admin: &dyn IReferencesSpaceAdminPort,
                     id_service: &dyn IdService) -> Result<RosterUid, CustomRosterError>
```

1. `admin.is_space_admin(…)` → `Forbidden`
2. engendrer les uid : `CUSTOM_<sulid>`, puis `<uid>__<SULID>` par poste
3. résoudre les limites croisées : les **index** de la commande deviennent les uid
4. `CustomRoster::try_new(draft)` → `Invalid(DomainError)` — carte 440
5. **vérifier l'existence** de chaque compétence, mot-clef, catégorie, staff et
   règle spéciale, contre `refs` — les cinq contrôles de `check_consistency`
6. `repo.save_custom_roster(…)`

**Les étapes 4 et 5 sont deux couches différentes.** Le domaine juge le roster
seul ; le use case juge ce qu'il référence. Le domaine ne peut pas faire la
seconde sans connaître un port.

**Les uid des postes viennent d'un identifiant engendré, pas d'un slug du nom** :
un slug casse au renommage, et deux postes homonymes produiraient le même uid.

**Aucun événement** : personne n'a à réagir à un roster qui apparaît.

## 2 · Modifier

Même chose, plus :

- `usage.count_teams_using(uid)` → **si > 0, `InUse { teams }`**
- le uid du roster est **conservé**, et ceux des postes qui subsistent

### Le verrou se re-vérifie ici

L'écran affiche « Modifier » sur un compteur à zéro. Entre l'affichage et
l'enregistrement, une équipe peut naître. **L'écran avertit, le serveur
tranche.**

### Les uid des postes survivent quand le poste survit

Sans conséquence tant que le roster n'est pas utilisé — et il ne l'est pas,
sinon on n'en serait pas là. **Mais ça comptera le jour où l'on autorisera la
modification d'un roster joué**, et changer les uid sous les pieds des joueurs
existants les détacherait de leur poste. Le faire correctement maintenant coûte
le même effort.

## 3 · Supprimer

```rust
1. accès, existence, appartenance
2. count_teams_using(uid) → si > 0, InUse { teams }
3. repo.delete_custom_roster(uid)
4. emettre(bus, ReferencesDomainEvent::CustomRosterDeleted { uid, space_id })
```

**`emettre()` et non `.send()`** — axe 12 de `check-arch`. Le helper est seul à
voir l'enveloppe produite, et `to_enveloppe()` engendre un identifiant : une
ligne écrite à la main au-dessus d'un `send` reprendrait celui de l'enveloppe
reçue et corrélerait n'importe quoi.

**Les tiers de compétition ne bloquent pas** : ils sont mis à jour après coup,
par la carte 444.

## Les erreurs

```rust
pub enum CustomRosterError {
    Forbidden,
    NotFound,
    InUse { teams: u32 },
    Invalid(DomainError),
    UsageUnavailable(String),
    Repository(String),
}
```

**`InUse` porte le nombre** : l'écran doit dire « 3 équipes le jouent », pas
« impossible ». Une erreur qui dit non envoie chercher.

**`UsageUnavailable` est distincte de `Repository`.** Si le port vers `teams`
échoue, on **ne sait pas** si le roster est utilisé — et on refuse. Traiter
l'indisponibilité comme un zéro laisserait supprimer un roster joué par cent
équipes parce qu'une requête a échoué. **Le doute ferme la porte.**

## Instrumentation

Les trois sont des `pub async fn` de `use_cases/` : `#[tracing::instrument(skip_all,
fields(cmd = ?cmd))]` obligatoire, sans quoi l'axe 11 de `check-arch` refuse.

## Tests

| Test | Règle |
|---|---|
| `un_non_admin_est_refuse` | P1 |
| `un_roster_d_un_autre_espace_est_introuvable` | P2 — `NotFound`, pas `Forbidden` |
| `une_competence_inconnue_est_refusee` | l'étape 5 |
| `les_index_de_limite_croisee_deviennent_des_uid` | la résolution |
| `modifier_un_roster_joue_est_refuse` | U1 |
| `supprimer_un_roster_joue_est_refuse` | U1 |
| `un_port_d_usage_en_echec_refuse` | **U3** — le test qu'on n'écrit pas spontanément |
| `la_suppression_emet_l_evenement` | la chaîne commence |
| `un_poste_conserve_son_uid_a_la_modification` | I2 |

## Checklist

- [ ] `references/use_cases/` et le bus interne dans le contexte
- [ ] Les trois use cases, instrumentés
- [ ] `IReferencesSpaceAdminPort` et son adapter
- [ ] Les cinq contrôles d'existence, repris de `check_consistency`
- [ ] `CustomRosterError` et ses six variantes
- [ ] Les neuf tests
- [ ] `make lint && make test && make check-arch`
