# L'écran du jet · Phase 5 : use cases

**Entrée** : `04-dtos.md` validé.

## Deux use cases : un nouveau, un modifié

## 1. `validate_dismissals_phase_use_case` — modifié

Il rend aujourd'hui `Result<(), …>`. Il rendra l'issue :

```rust
pub async fn execute(…) -> Result<ValidateDismissalsOutcome, ValidateDismissalsPhaseError>
```

**Rien d'autre ne change dans son corps.** `cloturer_la_phase` appelle déjà
`team.validate_dismissals_phase()`, qui décidera désormais de son événement ;
l'issue se lit sur celui-ci, sans second calcul :

```rust
let outcome = match lot.last() {
    Some(TeamDomainEvent::CostlyMistakesPhaseStarted { .. }) => Outcome::CostlyMistakes,
    _ => Outcome::ReadyToPlay,
};
```

**Un point vérifié plutôt que supposé** : `cloturer_la_phase` travaille sur
l'agrégat **chargé avant** l'application des renvois. La décision repose donc sur
`team.treasury` d'avant le lot — et c'est juste, parce qu'**un renvoi ne
rembourse rien** : la trésorerie ne bouge pas entre le chargement et la clôture.
Les recrutements, eux, ont été validés à la phase précédente et sont déjà dans
l'agrégat.

Si un jour un renvoi rendait de l'argent, cette hypothèse tomberait. La méthode
domaine devra alors recevoir la trésorerie d'après-lot, et non la lire dans
`self`.

Le nettoyage du panier ne change pas : le commentaire existant reste vrai, une
revalidation échouant sur `expect_phase(Dismissals)` quelle que soit la phase
suivante.

## 2. `apply_costly_mistakes_use_case` — nouveau

```rust
#[tracing::instrument(skip_all, fields(cmd = ?cmd))]
pub async fn execute(
    cmd: ApplyCostlyMistakesCommand,
    repo: &dyn ITeamRepository,
    dice: &dyn IDiceRoller,
) -> Result<CostlyMistakesOutcome, ApplyCostlyMistakesError>
```

### Orchestration

```
1. charger l'équipe                        → TeamNotFound
2. roll = dice.d6()
3. incident = incident_for(team.treasury, roll)          ← domaine
4. damage_dice = match incident.dice_needed() {          ← domaine
       None   => vec![],
       OneD3  => vec![dice.d3()],
       TwoD6  => { let (a, b) = dice.two_d6(); vec![a, b] }
   }
5. gp_lost = loss_for(incident, team.treasury, &damage_dice)   ← domaine
6. event = team.apply_costly_mistakes(roll, damage_dice, incident, gp_lost)?
                                                          → Domain (mauvaise phase)
7. repo.append(…)                          → Repository
8. info!(roll, ?damage_dice, ?incident, gp_lost, "erreurs coûteuses appliquées")
```

**Le use case ne décide de rien.** Il tire ce que le domaine lui demande, dans
l'ordre que le domaine impose. Les trois appels des étapes 3 à 5 sont des
fonctions pures ; l'étape 6 est la seule qui produise un fait.

**Le jet a lieu avant le chargement de la phase ?** Non — l'équipe est chargée
d'abord, et l'étape 6 refuse si la phase n'est pas `CostlyMistakes`. Un dé
inutile aura été tiré, ce qui n'a aucune conséquence : il n'est écrit nulle part.
L'ordre inverse — vérifier la phase avant de tirer — ferait la même chose en deux
lectures.

### Erreurs

```rust
pub enum ApplyCostlyMistakesError {
    TeamNotFound,
    Domain(DomainError),      // phase incorrecte — un second jet
    Repository(String),
}
```

**Aucune erreur pour « trésorerie insuffisante »** : sous 100 kPo, l'équipe n'est
jamais entrée dans cette phase, et l'étape 6 refuse de toute façon. La règle du
seuil vit dans `validate_dismissals_phase`, pas ici.

**Aucune erreur d'écrêtage** : `gp_lost` est borné par la trésorerie dans
`loss_for`, et `TreasuryMovement::debit` écrête en dernier ressort.

### Ce que le use case ne fait pas

- **Il ne vérifie aucun droit.** Le contrôle d'accès vit dans la couche web,
  comme le précédent du projet — `post_update_roster` appelle `can_spend_spp`
  avant d'entrer dans le use case.
- **Il ne redirige pas.** L'issue est un fragment ; le contrôleur en fait une
  réponse.
- **Il ne relit pas la trésorerie après coup.** Le solde restant se déduit du
  débit, et le grand livre en garde la trace dans la même transaction.

## L'issue rendue

```rust
pub struct CostlyMistakesOutcome {
    pub roll: u8,
    pub damage_dice: Vec<u8>,
    pub incident: IncidentType,
    pub gp_lost: Kpo,
    pub treasury_before: Kpo,
    pub treasury_after: Kpo,
}
```

Elle porte de quoi rendre le fragment **sans relire l'agrégat**. `treasury_before`
y figure parce que le calcul affiché en part — « la moitié de 345 » n'a de sens
qu'avec 345 sous les yeux.

## Le double clic, et ce qui le protège

Un second POST tombe sur `expect_phase(CostlyMistakes)` dans l'agrégat, qui n'est
plus dans cette phase depuis le premier — `CostlyMistakesApplied` repose
`ReadyToPlay`. **L'idempotence sort du modèle**, sans verrou ni jeton.

Le contrôleur en fait un 409, et non un 422 : la requête est bien formée, c'est
l'état qui a changé. C'est déjà ce que fait `edit_match_report` sur un rapport
publié.

## Règles métier à préciser en phase 6

- **La forme de la table.** Six tranches, quatre colonnes : un tableau de bornes
  parcouru, ou un `match` sur des plages ? Le premier se relit à côté du
  règlement ; le second se vérifie par le compilateur.
- **Ce que fait `incident_for` hors des bornes** — une trésorerie sous 100 kPo ne
  devrait jamais l'atteindre. Rendre `None` par défaut, ou refuser ?
- **L'arrondi de l'incident majeur** : `(treasury / 2) - (treasury / 2) % 5` sur
  des entiers. À 345, la moitié vaut 172 en division entière, l'arrondi 170 — le
  même résultat que 172,5 arrondi à 170. À vérifier sur un cas impair.
