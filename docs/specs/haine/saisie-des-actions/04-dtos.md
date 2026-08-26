# Saisie des actions — gain de la Haine · Phase 4 : contrats de données

**Entrée** : `03-back.md` validé.

## 1. DTO d'entrée — le formulaire d'action

`RecordActionForm` (`io/web/record_action_controller.rs`) gagne deux champs
optionnels. Il reste en primitives : c'est le contrat HTTP, et sa validation est
le travail du handler.

```rust
#[derive(Deserialize)]
pub struct RecordActionForm {
    pub turn: u8,
    pub player_id: String,
    pub player_type: String,
    pub action_type: String,
    pub injury_type: Option<String>,
    pub sequel_stat: Option<String>,
    pub hate_gained: Option<bool>,     // absent = non répondu
    pub hate_keyword: Option<String>,  // l'uid, seulement si hate_gained
}
```

**Émis par** : le `x-data` d'`action-panel-widget.html`, via `htmx.ajax('POST')`.
**Consommé par** : `post_action_step3` / `post_action_step4`, qui construisent la
commande.

`hate_gained` est un `Option<bool>` et non un `bool` : l'absence du champ signifie
« la question n'a pas été posée » — le cas des blessures qui ne donnent pas la
Haine — quand `Some(false)` signifie « posée, et la réponse est non ». Les deux
sont légitimes, et un `bool` nu les confondrait.

## 2. Value object — le mot-clef

```rust
#[nutype(
    sanitize(trim),
    validate(not_empty, len_char_max = 40, regex = UID_MOT_CLEF),
    derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Display, AsRef)
)]
pub struct HatredKeyword(String);
```

`UID_MOT_CLEF` vaut `^[A-Z][A-Z0-9_]*$` : ce sont des identifiants de corpus
(`DARK_ELF`, `BIG_GUY`), jamais du texte saisi. **Il ne passe donc pas par
`TEXTE_SAISI`**, qui gouverne les libellés que tape un humain.

**Le VO valide la forme, pas l'existence.** Un uid absent du corpus est
syntaxiquement correct ; c'est le use case qui le refuse, en consultant
`IKeywordCatalogPort`. Un value object qui interrogerait un port cesserait d'être
un objet du domaine.

## 3. La commande

`RecordActionCommand` ne change pas : la Haine voyage **dans l'action**, pas à
côté.

```rust
pub enum MatchActionType {
    Touchdown, Passe, Interception, Agression, Lancer, Sortie, Mvp,
    Blesse { injury: InjuryType, hatred: Option<HatredKeyword> },
}
```

`Option<HatredKeyword>` et non un couple `(bool, Option<…>)` : `None` dit
« aucune Haine », `Some(k)` dit laquelle. L'état « oui mais sans mot-clef » n'est
pas représentable — le front l'interdit déjà en masquant la confirmation, et le
type l'interdit ici pour de bon.

**Émise par** : le handler. **Consommée par** : `record_action_use_case`.

## 4. DTOs de port

### `IKeywordCatalogPort` — nouveau

```rust
pub struct KeywordDto {
    pub uid: String,
    pub label: String,
}

pub trait IKeywordCatalogPort: Send + Sync {
    fn list_all(&self) -> Vec<KeywordDto>;
    fn exists(&self, uid: &str) -> bool;
}
```

**Émis par** : `keyword_catalog_adapter` (infrastructure), depuis
`IReferenceRepository`.
**Consommé par** : `hate_keywords_service` pour le rendu, et
`record_action_use_case` pour `exists`. **Jamais par un handler ni un template**
— règle des domain services.

`list_all` rend un `Vec` et non un `&[KeywordDto]` : le port ne promet pas que le
catalogue vit en mémoire, même si l'implémentation actuelle le fait.

### `RosterPositionDto` — existant, un champ de plus

```rust
pub struct RosterPositionDto {
    pub position_uid: String,
    pub position_name: String,
    pub base_cost: u32,
    pub max_qty: u8,
    pub is_journeyman: bool,
    pub keywords: Vec<String>,   // les uids portés par la ligne de roster
}
```

**Émis par** : `ref_team_data_adapter`. **Consommé par** :
`hate_keywords_service`, et déjà par `record_inducements_use_case` pour les
mercenaires — qui ignorera simplement le nouveau champ.

## 5. Objet de domaine rendu par le service

```rust
// use_cases/hate_keywords_service.rs
pub struct HateKeywordChoices {
    pub in_opponent_roster: Vec<Keyword>,
    pub others: Vec<Keyword>,
}

pub struct Keyword {
    pub uid: String,
    pub label: String,
}
```

Les deux listes sont **triées par libellé**, jamais par uid. Le partage se fait
sur l'union des `keywords` du roster adverse.

**Émis par** : `hate_keywords_service::choices(opponent_team_id, team_data, catalog)`.
**Consommé par** : `action_panel_widget`, qui en fait ses VMs.

## 6. VMs de sortie

```rust
pub struct KeywordVm {
    pub uid: String,
    pub label: String,
}

pub struct ActionPanelTemplate {
    // … champs existants …
    pub opponent_keywords: Vec<KeywordVm>,
    pub other_keywords: Vec<KeywordVm>,
}
```

`KeywordVm::all_from_domain(&[Keyword]) -> Vec<KeywordVm>` — constructeur
co-localisé, le VM ne dépendant que du domaine local, jamais d'un DTO de port.

**Émis par** : `action_panel_widget`. **Consommé par** :
`action-panel-widget.html`, qui rend les deux grilles et laisse Alpine filtrer.

Les deux listes voyagent **en HTML rendu**, pas en JSON : le filtre travaille sur
le DOM déjà présent, comme dans la maquette. Aucun endpoint JSON, aucun appel au
moment du clic.

## 7. Ce qui traverse vers `players`

```rust
PlayerInjured {
    context: PlayerMatchContextPayload,
    injury_type: InjuryTypePayload,
    #[serde(default)]
    hatred_skill_uid: Option<String>,   // l'uid de la compétence, figé à la saisie
}
```

**Émis par** : le publisher de `match_report`, à la conclusion du match.
**Consommé par** : `player_match_impact_listener` dans `players`.

L'uid voyage **nu**, en `String` : c'est un app event, sérialisé et persisté, et
les value objects d'un BC ne traversent pas ses frontières.

**C'est l'uid de la compétence, pas celui du mot-clef.** Le corpus porte le lien
(`hate_skill_uid`), le use case le fige dans l'action au moment de la saisie, et
le publisher le recopie. Aucune convention de nommage, aucune résolution tardive
— et un fait passé ne dépend pas de l'état présent du référentiel.

## Règles métier tranchées

Les trois cas se ressemblent : dans chacun, un client envoie quelque chose
d'incohérent. **Ils sont tous refusés**, aucun n'est avalé.

| Cas | Réponse |
|---|---|
| `hate_keyword` inconnu du corpus | refus — le use case consulte `IKeywordCatalogPort::exists` |
| `hate_gained = Some(true)` sans `hate_keyword` | refus au handler, avant même de construire la commande |
| Haine envoyée sur une blessure qui n'en donne pas — Commotion, Mort | refus |

**Pourquoi refuser plutôt qu'ignorer.** Un échec avalé ne se voit pas : il
produit une donnée fausse et silencieuse. Le `CLAUDE.md` nomme déjà quatre
endroits où le projet le fait et le regrette — `UnknownSkill` qui accuse le
catalogue quand seul un nom était en cause, un poste replié sur « Joueur », un
roster escamoté deux fois par un `.ok()?`. Reproduire ce mécanisme sur la Haine
donnerait des blessures enregistrées sans le trait, sans une ligne de journal
pour le dire, et un coach persuadé d'avoir déclaré quelque chose qui n'existe
pas.

**La forme du refus** : `422 Unprocessable Entity`, comme les autres échecs de
validation du contrôleur d'action, avec une ligne en `warn` nommant le cas. Le
front rend ces trois cas impossibles à produire — la confirmation reste masquée,
la section ne s'ouvre pas sur une Commotion — donc un 422 ici signale un client
hors de l'interface, ce qui mérite précisément d'être vu.
