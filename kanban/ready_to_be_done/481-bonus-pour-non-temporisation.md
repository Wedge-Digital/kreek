# Le bonus pour non temporisation

**Épic :** aucune · **Dépend de :** rien
**Maquette :** `assets/rawpages/html/app-match-report-step5.html`

## Objectif

Une case à cocher sous chaque équipe, à l'étape 5 du rapport de match : **Bonus
pour non temporisation · +10 kPo**. Cochée, l'équipe gagne 10 kPo de plus sur ce
match.

## L'unité est le kPo, et la maquette ment sur ce point

Le domaine compte en **kPo**, pas en pièces d'or :

```rust
// match_report_pre_match.rs:391
let base = (fans_home + fans_away) / 2 * 10;      // 12 et 11 fans → 110
```

```rust
// le test qui le dit
// fans_home = 10+2=12, fans_away = 10+1=11, base = (12+11)/2 = 11 → 11 * 10 = 110 kPo
```

Le récapitulatif l'affiche déjà ainsi — `recap.html:269` :

```html
<span class="ms-stat-value ms-stat-value--gain">+{{ gains_fan.home_gain_kpo }} kPo</span>
```

**La maquette, elle, affiche `60000` avec `step="10000"`.** C'est un artefact de
maquette, antérieur à cette carte, et il ne doit **pas** être transcrit : le
bonus vaut `+10`, pas `+10000`. Un `+10000` sur un champ qui vaut `130`
multiplierait la trésorerie d'une équipe par soixante-dix.

## Le vrai choix : un booléen, pas un montant déjà additionné

On pourrait faire ajouter les 10 kPo par le contrôleur avant de construire le
`MatchGain`. Rien d'autre ne changerait. **Ça ne tient pas**, et pour une raison
mécanique, pas esthétique.

L'étape 5 **se relit** avec ses valeurs enregistrées :

```rust
// step5_controller.rs:168
rtp.home_gain.into_inner(),
```

Un montant déjà additionné donnerait donc, à la réouverture, un champ à 130 et
une case **décochée**. La recocher et enregistrer produirait 140. Puis 150.
**Le bonus se cumulerait à chaque correction**, sans que rien ne le signale —
et la zone de correction existe (`recap_controller.rs:313`).

Le booléen, lui, fait un aller-retour juste : le champ montre le montant saisi,
la case montre son état, et le total se recalcule.

## Le value object

```rust
// match_report/domain/value_objects.rs, section « step5 : après-match »
#[nutype(derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default))]
pub struct NoStallingBonus(bool);

impl NoStallingBonus {
    /// Le montant vit **avec le drapeau**, pas dans le contrôleur ni le gabarit.
    pub const AMOUNT_KPO: u32 = 10;
    pub fn amount_kpo(&self) -> u32 {
        if self.into_inner() { Self::AMOUNT_KPO } else { 0 }
    }
}
```

**Un newtype et non un `bool` nu**, contrairement à `is_journeyman` qui n'est
qu'un drapeau : celui-ci **porte un montant**. Le chiffre `10` apparaît sinon
dans le gabarit, dans le contrôleur et dans le calcul du total — trois endroits
à changer le jour où la ligue le passe à 20, et deux qu'on oubliera.

`IsStarPlayer` est le précédent maison d'un drapeau en newtype.

## Ce qui traverse la pile

```
formulaire  home_no_stalling_bonus / away_no_stalling_bonus  (bool)
    ▼
record_post_match(…, home_bonus: NoStallingBonus, away_bonus: NoStallingBonus, …)
    ▼
PostMatchRecorded { …, home_no_stalling_bonus, away_no_stalling_bonus }
    ▼
MatchReportReadyToPublish { …, les deux drapeaux }
    ▼
MatchReportPublished    →  app event  home_gain_kpo = gain + bonus
    ▼
teams  crédite la trésorerie du **total**
```

### Le total se calcule dans le domaine

```rust
impl MatchReportReadyToPublish {
    pub fn total_home_gain_kpo(&self) -> u32 {
        self.home_gain.into_inner() + self.home_no_stalling_bonus.amount_kpo()
    }
}
```

C'est « que se passe-t-il quand ? », donc le domaine — la grille du `CLAUDE.md`.
Le contrôleur ne fait pas d'arithmétique de gains.

### L'app event porte le total, pas le détail

`home_gain_kpo` reste le montant que `teams` crédite. **`teams` n'a pas à
connaître la règle du bonus** — il crédite ce qu'on lui dit. Le drapeau ne sort
pas de `match_report`, où il sert à réafficher l'écran et à raconter le match.

## Le piège du formulaire — une case décochée n'envoie rien

```rust
#[derive(Deserialize)]
pub struct RecordPostMatchForm {
    pub home_gain: u32,
    pub away_gain: u32,
    #[serde(default)]                       // ← sans ça, décocher rend un 422
    pub home_no_stalling_bonus: bool,
    #[serde(default)]
    pub away_no_stalling_bonus: bool,
    …
}
```

Un `<input type="checkbox">` non coché **n'est pas envoyé du tout** — le
navigateur ne poste pas `false`, il ne poste rien. Sans `#[serde(default)]`, le
cas le plus courant (personne ne prend le bonus) échouerait à la
désérialisation, et l'étape 5 refuserait de s'enregistrer.

## Le gabarit

`step5.html`, section « Gains du match ». Chaque équipe passe d'une
`.mr-gains-team-row` nue à un bloc :

```html
<div class="mr-gains-team">
  <div class="mr-gains-team-row"> … existant, inchangé … </div>
  <label class="mr-gains-bonus">
    <input type="checkbox" class="mr-gains-bonus-check"
           name="home_no_stalling_bonus" {% if home_no_stalling_bonus %}checked{% endif %}>
    <span class="mr-gains-bonus-label">Bonus pour non temporisation</span>
    <span class="mr-gains-bonus-amount">+10 kPo</span>
  </label>
</div>
```

**Le fond `--dark-7` remonte de la ligne au bloc.** Laissé sur la ligne, la case
flotterait sur du blanc, détachée de l'équipe qu'elle concerne.

### Le CSS

`pages/match-report-step5.css` — trois classes neuves, `.mr-gains-team`,
`.mr-gains-bonus` et ses deux enfants. La case suit la convention maison :
`18px`, `accent-color: var(--main-blue)`, comme `bonus-check` du magicien de
compétition.

La pastille de montant passe du gris au bleu **en CSS pur**, par
`.mr-gains-bonus-check:checked ~ .mr-gains-bonus-amount`. Pas de JS : c'est de
l'état d'écran dérivé d'une case, exactement ce qu'un sélecteur sait faire.

### Ce que le JS de la maquette ne devient pas

La maquette recalcule le montant du champ à la volée pour démontrer la règle.
**En production, non** : le champ reste le montant de base saisi par l'arbitre,
et le total ne se voit qu'au récapitulatif. Recalculer le champ ferait
exactement l'erreur que cette carte évite — un montant additionné qu'on relit
comme une base.

## Le récapitulatif

`recap.html:269` affiche `+{{ gains_fan.home_gain_kpo }} kPo`. Le VM porte déjà
le total, donc **la ligne est juste sans y toucher**.

Reste à décider si le récapitulatif **mentionne** le bonus. Je pose que oui,
d'une ligne discrète sous le montant — sinon un coach qui compare le gain à la
formule ne retrouve pas ses 10 kPo, et il n'a aucun moyen de savoir d'où ils
viennent.

## Tests

### Unitaires

| Test | Ce qu'il prouve |
|---|---|
| `un_bonus_actif_ajoute_dix_kpo_au_total` | la règle |
| `un_bonus_inactif_n_ajoute_rien` | et son contraire |
| `le_gain_enregistre_reste_le_montant_saisi` | **le total n'écrase pas la base** |
| `les_deux_equipes_ont_des_bonus_independants` | pas de fuite d'un côté à l'autre |
| `l_evenement_post_match_porte_les_deux_drapeaux` | la persistance |
| `l_app_event_porte_le_total_et_non_la_base` | ce que `teams` crédite |

`le_gain_enregistre_reste_le_montant_saisi` est celui qui compte : il fixe la
décision qui empêche le cumul à la correction.

### Intégration ou e2e

| Test | Ce qu'il prouve |
|---|---|
| `relire_l_etape_5_retrouve_la_case_cochee_et_le_montant_de_base` | **l'aller-retour** |
| `enregistrer_deux_fois_ne_cumule_pas_le_bonus` | le défaut qu'on évite |
| `une_case_decochee_ne_fait_pas_echouer_l_enregistrement` | le `#[serde(default)]` |

Le deuxième se fait en enregistrant, revenant à l'étape 5, et réenregistrant
sans rien toucher : le total doit être identique.

## Ce que la carte ne fait pas

- **Aucune règle automatique.** L'arbitre coche, l'application ne décide pas
  toute seule qu'il n'y a pas eu temporisation.
- **Aucun réglage de compétition** : le bonus vaut 10 kPo partout, il ne
  s'active ni ne se paramètre par ligue. `AMOUNT_KPO` est là pour le jour où.
- **Aucune correction de l'unité de la maquette** — son `60000` reste faux,
  c'est un autre sujet.

## Checklist

- [ ] `NoStallingBonus` avec `AMOUNT_KPO` et `amount_kpo()`
- [ ] `record_post_match` prend les deux drapeaux
- [ ] `PostMatchRecorded`, `MatchReportReadyToPublish`, `MatchReportPublished` les portent
- [ ] `total_home_gain_kpo()` / `total_away_gain_kpo()` **dans le domaine**
- [ ] L'app event `home_gain_kpo` / `away_gain_kpo` porte le **total**
- [ ] `RecordPostMatchForm` avec `#[serde(default)]` sur les deux
- [ ] `step5.html` : le bloc, la case, l'état `checked` au retour
- [ ] `pages/match-report-step5.css` : les trois classes, pastille en CSS pur
- [ ] **Aucun `step="10000"`, aucun `+10000`** — l'unité est le kPo
- [ ] Les six tests unitaires, les trois d'aller-retour
- [ ] `make lint && make test && make check-arch && make e2e`
