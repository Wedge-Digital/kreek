# L'écran du jet · Phase 4 : contrats de données

**Entrée** : `03-back.md` validé.

## 1. DTO d'entrée — il n'y en a pas

Le POST du jet **n'a pas de corps**. Le client demande qu'on tire un dé ; il
n'en propose pas un.

```
POST /app/{space_id}/teams/{team_id}/costly-mistakes/roll
```

L'identité de l'équipe est dans le chemin, celle du coach dans la session. Rien
d'autre n'entre, donc rien n'est à valider — et rien ne peut être forgé.

**Émis par** : le composant Alpine qui tient l'animation, via `htmx.ajax`.
**Consommé par** : `post_roll_costly_mistakes`.

## 2. La commande

```rust
pub struct ApplyCostlyMistakesCommand {
    pub team_id: EntityId,
}
```

Un seul champ, et c'est tout ce que le use case a besoin de savoir : la
trésorerie vient de l'agrégat, les jets viennent du port.

**Le jet n'est pas dans la commande.** Le mettre reviendrait à le laisser entrer
par la frontière HTTP un jour ou l'autre — c'est précisément ce qu'on évite.

**Émise par** : le handler. **Consommée par** : `apply_costly_mistakes_use_case`.

## 3. Le port du hasard

```rust
pub trait IDiceRoller: Send + Sync {
    fn d6(&self) -> u8;
    fn d3(&self) -> u8;
    fn two_d6(&self) -> (u8, u8);
}
```

Aucun DTO : le port rend des entiers. `two_d6` rend un **couple** et non un
`Vec` — deux dés, toujours, et le type le dit.

**Émis par** : `dice_adapter` (infrastructure), sur `rand`.
**Consommé par** : `apply_costly_mistakes_use_case` seul. Ni le domaine, ni le
handler, ni un template.

## 4. Les types du domaine

```rust
// domain/costly_mistakes.rs — pur, aucune dépendance
pub enum DiceNeeded { None, OneD3, TwoD6 }

pub fn incident_for(treasury: Kpo, roll: u8) -> IncidentType;
pub fn loss_for(incident: IncidentType, treasury: Kpo, dice: &[u8]) -> Kpo;

impl IncidentType { pub fn dice_needed(&self) -> DiceNeeded; }
```

`IncidentType { None, Minor, Major, Catastrophe }` **existe déjà**
(`teams/domain/value_objects.rs:110`), et ses quatre variants sont exactement les
quatre colonnes de la table.

`loss_for` prend `&[u8]` plutôt qu'un type par incident : le nombre de dés est
déjà décidé par `dice_needed`, et trois signatures pour un même calcul de perte
coûteraient plus qu'elles ne prouvent. Un jeu de dés qui ne correspond pas à
l'incident est un bug d'appelant, pas un cas métier.

## 5. L'événement

```rust
CostlyMistakesApplied {
    roll: u8,
    #[serde(default)]
    damage_dice: Vec<u8>,
    incident: IncidentType,
    gp_lost: Kpo,
}
```

**Émis par** : `Team::apply_costly_mistakes`. **Consommé par** : `apply()` de
l'agrégat, la projection, le grand livre, et les deux listeners qui recalculent
la valeur d'équipe et purgent les paniers.

`gp_lost` est le montant **décidé**. Le montant **effectif** est calculé par
`TreasuryMovement::debit`, qui écrête au solde — les deux ne peuvent pas diverger
ici, chaque effet étant borné par la trésorerie, mais c'est le débit qui fait foi
au grand livre.

## 6. L'issue de la validation des renvois

```rust
pub enum ValidateDismissalsOutcome {
    ReadyToPlay,
    CostlyMistakes,
}
```

**Émise par** : `validate_dismissals_phase_use_case`.
**Consommée par** : `post_validate_dismissals_phase`, qui choisit sa redirection
— la fiche d'équipe, ou la page du jet.

Une énumération et non un `bool` : `ReadyToPlay` et `CostlyMistakes` sont deux
destinations, pas la présence ou l'absence d'une chose. Le jour où une troisième
phase s'intercale, elle s'ajoute ici sans qu'on ait à relire les appelants.

## 7. Les VMs de sortie

```rust
pub struct CostlyMistakesTemplate {
    pub team: TeamHeaderVm,          // nom, initiales, roster — comme les autres pages de phase
    pub treasury_kpo: u32,
    pub band: BandVm,                // la tranche du coach
    pub table: Vec<BandVm>,          // les six lignes
    pub roll_url: String,
    pub back_url: String,
}

pub struct BandVm {
    pub label: String,               // « 300 – 399 kPo »
    pub safe: String,                // « 4–6 »
    pub minor: String,
    pub major: String,
    pub catastrophe: String,
    pub is_current: bool,
}
```

```rust
pub struct CostlyMistakesResultTemplate {
    pub roll: u8,
    pub incident_css: String,        // safe / minor / major / catastrophe
    pub incident_label: String,      // « Incident majeur »
    pub incident_text: String,
    pub calc: Vec<CalcLineVm>,       // les lignes du calcul, dans l'ordre
    pub table: Vec<BandVm>,          // réaffichée, avec la case touchée
    pub hit_column: String,
    pub continue_url: String,
}

pub struct CalcLineVm {
    pub label: String,
    pub value: String,
    pub dice: Vec<u8>,               // affichés en petit à côté, vide si aucun
    pub style: String,               // normal / total / rest
}
```

**Émis par** : `costly_mistakes.rs`. **Consommés par** : les deux templates.

**Le calcul est une liste de lignes, pas quatre champs nommés.** Chaque incident
a son propre enchaînement — un majeur montre la moitié puis l'arrondi, une
catastrophe montre ce qui est sauvé — et un VM à champs fixes obligerait le
template à savoir lequel afficher. La liste vient du serveur, le template la
déroule.

`table` est renvoyée **avec le fragment** : le résultat surligne une case, et
réafficher les six lignes coûte moins qu'un second échange pour mettre à jour un
tableau.

## Règles métier tranchées en phase 4

| Question | Décision |
|---|---|
| `gp_lost` supérieur au solde ? | impossible — chaque effet est borné par la trésorerie. `TreasuryMovement::debit` écrête de toute façon, et c'est lui qui fait foi au livre |
| Le jet est-il journalisé ? | **oui** — un `info!` portant `roll`, `damage_dice`, `incident` et `gp_lost`, sur cible `kreek::`. Une contestation doit être vérifiable sans ouvrir l'event store |
| Le client peut-il proposer un jet ? | **non**, le POST n'a pas de corps |
