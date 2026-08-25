# Saisie des actions — gain de la Haine · Phase 6 : domaine

**Entrée** : `05-use-cases.md` validé.

## Récapitulatif exhaustif des règles métier — validé

| # | Règle | Où elle vit |
|---|---|---|
| R1 | La Haine se gagne sur **trois blessures** : Amoché, Blessure Sérieuse, Séquelle. Ni Commotion, ni Mort | domaine `match_report` |
| R2 | Le gain est **déclaré** par le coach, jamais automatique | front + commande |
| R3 | Une Haine est qualifiée par **exactement un** mot-clef | type `Option<HatredKeyword>` |
| R4 | Le mot-clef doit **exister au catalogue** | use case, via `IKeywordCatalogPort` |
| R5 | Les mots-clefs du **roster adverse** viennent en premier ; tous restent accessibles | `hate_keywords_service` |
| R6 | La Haine est **gratuite** : aucun SPP payé, aucune valeur ajoutée | domaine `players` |
| R7 | **Aucune gestion de doublon** — deux fois le même mot-clef est permis | nulle part, volontairement |
| R8 | Le **cumul** de Haines différentes n'est pas borné | nulle part |
| R9 | Un **journalier** peut en gagner une ; elle reste dans le rapport | publisher, phase 7 |
| R10 | Acquise **à la publication**, défaite **à la dépublication** | mécanisme d'impact existant |
| R11 | **Supprimer l'action** supprime la Haine | mécanique : rien n'est parti avant publication |
| R12 | Le mode d'acquisition est **`Injury`** | `players` |
| R13 | Toute incohérence est **refusée en 422**, jamais avalée | handler, use case, domaine |
| R14 | La Haine n'est **pas achetable en SPP** | acquis : `TRAITS` n'est dans l'accès d'aucun poste |

**R7 et R8 ne vivent nulle part, et c'est une décision** : ne rien écrire est ici
le travail. Une règle « pas de doublon » aurait demandé une consultation de
`players` depuis la saisie, un refus dans le domaine, et leurs tests.

## Ce que la phase 5 disait, et qui est corrigé ici

La phase 5 plaçait le contrôle de R1 dans le use case **et** laissait entendre
que le domaine le referait. Un invariant tenu à deux endroits diverge : le jour
où l'un change, l'autre ment.

**Seul `record_action` porte R1.** Le use case ne garde que R4 — la consultation
du catalogue, qui ne peut pas vivre dans le domaine puisqu'elle passe par un
port — et traduit `DomainError` en 422.

## `match_report` — l'invariant et son gardien

```rust
impl InjuryType {
    /// Amoché, Blessure Sérieuse, Séquelle. Une Commotion est trop légère pour
    /// laisser une rancune, une Mort ne laisse personne pour haïr.
    pub fn peut_donner_haine(&self) -> bool {
        matches!(self, Self::Amoche | Self::BlessureSerieuse | Self::Sequel { .. })
    }
}
```

**C'est la source unique de R1.** La constante `PEUT_GAGNER_HAINE` du template
n'en est qu'un reflet destiné à masquer la section — elle ne décide de rien, et
un écart entre les deux se solde par un 422, pas par une donnée fausse.

```rust
impl MatchReportPreMatch {
    pub fn record_action(
        self, side: TeamSide, turn: TurnNumber, player: ActionPlayer,
        action: MatchActionType, display_name: String, position: String,
        action_id: ActionId, recorded_by: CoachId,
    ) -> Result<(Self, MatchReportDomainEvent), DomainError>
}
```

**La signature change** : elle rendait un couple, elle rend désormais un
`Result`. C'est R1 qui l'impose — une action pouvait jusqu'ici toujours être
enregistrée, ce n'est plus vrai.

```rust
MatchActionType::Blesse { injury, hatred: Some(_) } if !injury.peut_donner_haine()
    => return Err(DomainError::HatredNotAllowedForInjury),
```

Toutes les autres actions passent sans changement, et le `Result` se propage
jusqu'au use case, qui le mappe sur `RecordActionError::Domain`.

### L'agrégat fuit, et il faut le savoir

Les vingt champs de `MatchReportPreMatch` sont `pub` — `home_actions`,
`away_actions`, `home_team_value` — et `record_inducements_use_case` mute
directement `pm.home_team_value`. R1 sera donc gardée par `record_action` et
**restera contournable** en écrivant dans le vecteur d'actions.

La Haine n'aggrave rien : elle hérite d'un état de fait. Ce constat est écrit
parce que la phase 6 demande de le vérifier, pas pour élargir ce chantier — la
fermeture de cet agrégat est un sujet en soi.

## `players` — le trait gratuit

L'agrégat **ne gagne aucun champ**. La Haine est une compétence acquise, donc
elle entre dans `acquired_skills`, avec le précédent exact des compétences
customisées (`player.rs:523`) :

```rust
AcquiredSkill {
    skill_id: SkillId::try_new(format!("HAINE_{uid}"))?,
    skill_name,                       // « Haine : Nain », résolu au catalogue
    mode: AcquisitionMode::Injury,
    // Ni coût ni valeur : un trait gagné en encaissant un coup ne se paie pas
    // et ne renchérit pas le joueur.
    spp_cost: SppCost::try_new(0).unwrap(),
    value_delta: ValueKpo(0),
}
```

**La nuance que la phase 4 laissait ouverte est levée** : « il n'existe pas, il
ne vaut pas zéro » vaut pour **l'événement**, qui ne portera pas de champ de
valeur — comme `PlayerSkillCustomised`. L'**état projeté**, lui, porte des zéros,
et c'est déjà le cas pour les compétences customisées.

```rust
pub enum AcquisitionMode { Chosen, Random, Customised, Injury }

impl Player {
    /// Le trait gagné en encaissant. Aucun SPP n'est dépensé, aucune valeur
    /// n'est ajoutée : la méthode ne consulte donc ni la réserve, ni le barème.
    pub fn record_hatred(
        &self, context: MatchContext, keyword: HatredKeyword, skill_name: SkillName,
    ) -> PlayerDomainEvent
}
```

Elle ne rend pas de `Result` : à ce stade, tout a été vérifié en amont — R4 par
le use case, R1 par `match_report` — et R7 dit qu'il n'y a pas de doublon à
refuser. Une méthode qui rendrait `Result` sans jamais échouer ferait écrire des
`unwrap` à ses appelants.

## Erreurs domaine

```rust
// match_report
DomainError::HatredNotAllowedForInjury
```

Une seule. `UnknownKeyword` reste applicatif : le domaine n'a pas de catalogue,
et lui en donner un l'obligerait à connaître un port.

## Tests unitaires — un par règle

| Test | Règle |
|---|---|
| Haine sur Amoché, Blessure Sérieuse, Séquelle → acceptée | R1 |
| Haine sur Commotion → `HatredNotAllowedForInjury` | R1 |
| Haine sur Mort → `HatredNotAllowedForInjury` | R1 |
| Blessure sans Haine sur les cinq types → acceptée | R1 |
| Les six autres actions inchangées par la signature en `Result` | non-régression |
| `peut_donner_haine` sur les cinq variants, `Sequel` quelle que soit la stat | R1 |
| Deux fois le même mot-clef sur un joueur → accepté | R7 |
| Trois Haines différentes → acceptées | R8 |
| `record_hatred` → `spp_cost` 0, `value_delta` 0, mode `Injury` | R6, R12 |
| La réserve de SPP du joueur est inchangée après un gain | R6 |
| `HatredKeyword` : `DARK_ELF` accepté, `dark elf` refusé, vide refusé | forme |

**Le quatrième test compte autant que les trois premiers** : R1 interdit la
*Haine* sur une Commotion, pas la Commotion. Sans lui, une implémentation qui
refuserait toute blessure légère passerait les trois premiers.
