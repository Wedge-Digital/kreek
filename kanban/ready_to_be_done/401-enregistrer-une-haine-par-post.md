# Enregistrer une Haine par POST

**Priorité : haute**
**Dépend de :** 399 et 400
**Conception :** `docs/specs/haine/saisie-des-actions/{04-dtos,05-use-cases}.md`
**Fichiers :** `src/app/match_report/ports.rs`,
`src/infrastructure/match_report/keyword_catalog_adapter.rs`,
`src/app/match_report/use_cases/record_action_use_case.rs`,
`io/web/record_action_controller.rs`, `context.rs`, `src/main.rs`

## Objectif

Le POST d'action accepte deux champs de plus, et refuse ce qui n'a pas de sens.
Sans interface : à la fin de cette carte, la Haine s'enregistre par requête, et
la carte 402 lui donne un écran.

```rust
pub struct RecordActionForm {
    // …
    pub hate_gained: Option<bool>,
    pub hate_keyword: Option<String>,
}
```

`hate_gained` est un `Option<bool>` : l'absence dit « la question n'a pas été
posée » — le cas d'une Commotion — quand `Some(false)` dit « posée, et la réponse
est non ». Un `bool` nu confondrait les deux.

## Le port du catalogue

```rust
pub struct KeywordDto {
    pub uid: String,
    pub label: String,
    pub hate_skill_uid: String,   // le corpus le porte, on ne le déduit pas
}

pub trait IKeywordCatalogPort: Send + Sync {
    /// Les mots-clefs **haïssables**, et eux seuls.
    fn list_hateable(&self) -> Vec<KeywordDto>;
    fn find_hateable(&self, uid: &str) -> Option<KeywordDto>;
}
```

`find_hateable` rend le DTO **avec son `hate_skill_uid`** : le use case l'a donc
en main au moment où il valide, et le fige dans l'action. Personne n'aura à le
résoudre plus tard.

**Le port ne rend jamais les huit autres.** Un mot-clef de poste — Blitzer,
Trois-quart — existe au corpus mais ne se hait pas ; le faire filtrer par chaque
appelant serait la garantie qu'un l'oublie. Le DTO ne porte donc pas de drapeau :
son existence dans la réponse **est** le drapeau.

**Un port dédié, et non une méthode de plus sur `ITeamDataPort`** : celui-ci
répond « que sait-on de cette équipe ? », celui-là « quels mots-clefs le
règlement connaît-il ? ». Les fondre obligerait à passer un `team_id` à une
question qui n'en a pas.

L'adapter vit dans `infrastructure/match_report/` et lit `IReferenceRepository`.

## Les trois refus

| Cas | Qui refuse | Erreur |
|---|---|---|
| `hate_gained = Some(true)` sans mot-clef | le **handler**, avant la commande | 422 |
| uid absent du catalogue, **ou non haïssable** | le **use case**, via `find_hateable` | `UnknownKeyword(uid)` → 422 |
| blessure qui ne donne pas la Haine | le **domaine** (carte 400) | `HatredNotAllowedForInjury` → 422 |

**Le premier ne peut pas descendre plus bas** : la commande porte
`Option<HatredKeyword>`, donc l'état « oui sans lequel » n'est pas
représentable. Le typage a déplacé la validation là où on ne peut plus l'oublier.

**Aucun n'est avalé.** Le `CLAUDE.md` nomme quatre endroits où le projet avale un
échec et le regrette — `UnknownSkill` accusant le catalogue quand seul un nom
était en cause, un poste replié sur « Joueur », un roster escamoté deux fois par
un `.ok()?`. Une blessure enregistrée sans sa Haine, sans un mot au journal,
serait le cinquième.

**Un mot-clef non haïssable est refusé comme un inconnu**, et c'est délibéré :
l'écran ne le propose pas, donc une requête qui le porte vient d'ailleurs. Lui
inventer une erreur distincte reviendrait à documenter au client une nuance dont
il n'a pas à connaître l'existence.

`UnknownKeyword` **porte l'uid** : sans lui, le journal dirait qu'un mot-clef a
été refusé sans dire lequel, et le premier corpus incomplet coûterait une
investigation. `HatredNotAllowedForInjury` ne porte rien — le type de blessure
est déjà dans le champ `cmd` du span.

## L'ordre d'orchestration

On ne consulte le catalogue que si une Haine est déclarée, et on refuse avant de
charger l'agrégat quand c'est possible. Le use case décide de l'**ordre** ; la
règle, elle, reste au domaine.

## Checklist

- [ ] `IKeywordCatalogPort` + `KeywordDto` dans `ports.rs`
- [ ] `keyword_catalog_adapter.rs`, câblé dans `main.rs` et `MatchReportContext`
- [ ] `RecordActionForm` gagne deux champs
- [ ] `record_action_use_case` prend le port, gagne `UnknownKeyword`
- [ ] Le contrôleur refuse le gain sans mot-clef, mappe les trois cas en 422
      avec une ligne `warn`
- [ ] Tests unitaires :
  - [ ] uid inconnu → `UnknownKeyword`, aucune écriture
  - [ ] uid **existant mais non haïssable** (`BLITZER`) → `UnknownKeyword` aussi
  - [ ] gain sans mot-clef → 422 avant appel du use case
  - [ ] Haine sur Commotion → 422, propagé du domaine
  - [ ] chemin nominal → action enregistrée, **le mot-clef et la compétence**
        dans l'événement
  - [ ] action sans Haine → le catalogue **n'est pas consulté**
- [ ] `make lint`, `make check-arch`, `make test`
