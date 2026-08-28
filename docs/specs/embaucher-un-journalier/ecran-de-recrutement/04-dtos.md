# Écran de recrutement · Phase 4 : contrats de données

**Phase 3** : `03-back.md`

## Entrée — une route de plus, un DTO de moins

Le recrutement d'un journalier n'a **pas de corps** : la cible est dans le
chemin.

```
POST /app/{space_id}/teams/{team_id}/recruitment/journeyman/{player_id}
```

**Aucun `Deserialize`.** Il n'y a rien à choisir — pas de quantité, pas
d'option : ce journalier-là, ou aucun.

**Le `{player_id}` dans le chemin, jamais dans le corps.** C'est la leçon de la
carte 416, où `delete_match` prend sa cible dans le corps, hors de portée de
`space_scope`. Ici `{team_id}` est résolu par le middleware, et le use case
vérifie que le joueur appartient bien à cette équipe.

Le retrait du panier emprunte la route existante — une ligne de panier se retire
par son `BasketLineId`, quelle que soit sa nature.

## Ce que `players` transmet à `teams`

### `SquadMemberDto` gagne deux champs

```rust
pub struct SquadMemberDto {
    pub player_id: String,
    pub roster_line_id: String,
    pub jersey: Option<u8>,
    pub personal_name: String,
    pub position_name: String,
    pub spp: u32,
    pub value_kpo: u32,
    pub available_for_next_match: bool,
    pub is_temporary: bool,                    // ← membership == Journeyman
    pub improvement_label: Option<String>,     // ← « Blocage », « +1 ST »
}
```

**`is_temporary` et non `is_journeyman`** : ce dernier existe déjà sur
`RosterPositionDto` et signifie « ce poste est la ligne journalière du roster ».
Deux homonymes contradictoires dans le même BC seraient une confusion assurée.

**`improvement_label` est un libellé déjà composé**, pas une structure. Le DTO
est un contrat de lecture : `players` sait dire « Blocage » ou « +1 ST », et
`teams` n'a pas à savoir qu'une compétence et une caractéristique se rendent
différemment.

Il vient de deux sources, dans cet ordre :

```
acquired_skills[0].skill_name        →  « Blocage »
sinon un delta non nul               →  « +1 ST »
sinon                                →  None
```

**Une amélioration au plus** : le journalier n'a joué qu'un match, et le premier
palier coûte 6 SPP. Si les deux existaient — cas impossible aujourd'hui — la
compétence l'emporte, parce qu'elle se nomme.

### La lecture qui les remplit

```sql
SELECT player_id, roster_line_id, jersey, personal_name, position_name,
       spp, value_kpo, participation_status,
       membership,                                    -- ← neuf
       acquired_skills, ma_delta, st_delta, ag_delta, pa_delta, av_delta  -- ← neuf
FROM   players_proj
WHERE  team_id = $1 AND membership <> 'Dismissed'      -- ← était = 'Active'
ORDER BY jersey NULLS LAST, player_id
```

Mesuré : sur 49 000 joueurs, **298 ont une compétence acquise et 47 une
caractéristique améliorée**. Le coût de ces colonnes est négligeable, et la
requête sert déjà tout l'écran.

## Le panier

### Trois variantes, désormais

```rust
pub enum BasketLine {
    Player     { id: BasketLineId, roster_line: RosterLineId, price: Kpo },
    Staff      { id: BasketLineId, staff_type: StaffType, price: Kpo },
    Journeyman { id: BasketLineId, player_id: PlayerId, price: Kpo },   // ← neuf
}

pub enum AppliedLine {
    Player     { roster_line: RosterLineId, base_value: Kpo, cost: Kpo },
    Staff      { staff_type: StaffType, cost: Kpo },
    Journeyman { player_id: PlayerId, cost: Kpo },                      // ← neuf
}
```

**`AppliedLine::Journeyman` n'a pas de `base_value`.** Un recrutement ordinaire
distingue ce que le joueur vaut de ce qu'on paie ; ici les deux sont le même
nombre — le prix **est** la valeur courante (décision 9). Ajouter un champ qui
duplique l'autre inviterait à les faire diverger.

### Ce que le panier doit recevoir

`RecruitmentBasket` porte déjà `catalog`, `squad`, `owned_staff`, `treasury`. Il
lui faut la liste des recrutables :

```rust
pub struct HireableJourneyman {
    pub player_id: PlayerId,
    pub price: Kpo,
}
```

**Juste ce qu'il faut pour valider** : l'identifiant pour reconnaître, le prix
pour débiter. Le nom, les SPP et l'amélioration sont de l'affichage — ils vont
au view model, pas au domaine.

**C'est cette liste qui porte le garde-fou** (décision 14) : un `player_id` du
panier absent d'ici est une ligne rejetée. Le domaine reste pur — il compare le
panier à ce qu'on lui a donné, sans rien interroger.

### Une cause d'erreur de plus

```rust
pub enum DomainError {
    …,
    JourneymanNoLongerAvailable,   // ← neuf
}
```

Affichée « n'est plus disponible » — au même endroit et de la même façon que
« quota atteint » ou « trésorerie insuffisante ». L'écran ne distingue pas les
rejets par leur origine.

## Sortie — le view model

```rust
pub struct JourneymanRowVm {
    pub player_id: String,
    pub name: String,
    pub position_name: String,
    pub spp: u32,
    /// `None` → « aucune ». Le gabarit ne teste pas une chaîne vide.
    pub improvement: Option<String>,
    pub price_kpo: u32,
    /// Le tarif du poste, pour décomposer le prix quand il le dépasse.
    pub base_price_kpo: u32,
    pub action: ActionVm,
}
```

**`ActionVm` est réutilisé tel quel.** Il porte déjà les trois états —
`Enabled`, `Blocked`, `Forbidden` — et son `from_domain` traduit une
`ActionState` du domaine. La ligne de journalier n'invente rien.

**`base_price_kpo` sert à la décomposition** : le gabarit affiche « 65 + 20
d'amélioration » quand `price_kpo > base_price_kpo`. Le calcul de l'écart est
fait au rendu et non stocké — c'est une soustraction, pas une donnée.

**Pas de `is_last_chance` ni d'indicateur d'urgence** : l'urgence est portée par
le panneau, une fois, pas répétée sur chaque ligne.

### Le catalogue les porte

```rust
pub struct RecruitmentCatalogVm {
    …,
    pub journeymen: Vec<JourneymanRowVm>,   // ← vide si aucun
}
```

**Vide et non `Option`** : le gabarit teste `if !journeymen.is_empty()` pour
masquer le panneau, et une collection vide dit exactement la même chose qu'une
absence sans obliger à déballer.

### Construit dans `builders.rs`, pas par `from_domain`

`PositionRowVm::all_from_domain(&basket)` existe parce qu'un poste ne dépend que
du panier. Une ligne de journalier dépend **du panier et du DTO de port** — le
nom, les SPP et l'amélioration viennent de `SquadMemberDto`.

C'est la règle du `CLAUDE.md` : les VMs qui dépendent d'un port se construisent
dans `builders.rs`.

## Les deux app events

```rust
// match_report → teams
MatchReportAppEvent::JourneymenFielded {
    event_id, match_report_id, team_id, space_id,
    players: Vec<FieldedJourneyman>,
}
pub struct FieldedJourneyman {
    pub player_id: String,
    pub roster_line_id: String,
}

// teams → players
TeamsAppEvent::JourneymanRecruited {
    event_id, team_id, space_id, player_id,
}
```

**`FieldedJourneyman` ne porte pas de maillot** : `players` l'attribue à la
création, par `premier_libre`, qui seul connaît les numéros pris.

**`JourneymanRecruited` ne porte ni prix ni SPP.** C'est le principe déjà écrit
sur `PlayerDismissed` : *« `players` possède le joueur, il sait tout de lui ; ce
qu'il ignorait, c'est la décision. »* Le prix a servi à débiter `teams`, il n'a
rien à faire dans l'événement.

## Règles métier à préciser

Une seule, et elle est mineure.

**Que faire si un journalier porte une amélioration de caractéristique *et* une
compétence ?** Impossible aujourd'hui — un match ne donne pas assez de SPP —
mais le contrat doit trancher. J'ai posé : la compétence l'emporte, parce
qu'elle a un nom. À confirmer, ou à remplacer par un libellé qui les cumule.
