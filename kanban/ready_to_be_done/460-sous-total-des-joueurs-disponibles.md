# Sous-total des joueurs disponibles

**Priorité : moyenne** — le chiffre existe dans l'en-tête de l'équipe, mais rien
ne permet de le vérifier ligne à ligne
**Périmètre : le widget joueurs du BC `players`**
**Dépend de :** rien
**Maquette :** `assets/rawpages/html/app-team-detail.html`

## Objectif

Une ligne de pied sous le tableau des joueurs :

```
7 joueurs disponibles — 1 absent, hors du compte      —      603 kPo
```

C'est ce qui compose la valeur d'équipe du prochain match, et le chiffre qu'un
coach vérifie avant de jouer.

## La carte porte deux choses, et la seconde n'était pas demandée

**Le tableau ne distingue pas aujourd'hui les disponibles des indisponibles.**
`PlayerProjection` porte bien `participation_status` — `Available` ou
`MissingNextGame` — et la requête le lit déjà
(`projection_repository.rs:27`). Mais `PlayerRowVm` ne le reprend pas, et rien
ne l'affiche.

Un sous-total qui exclut une ligne **sans dire laquelle** paraîtrait faux à qui
additionne de l'œil. Le marquage n'est donc pas un supplément : il est ce qui
rend le total vérifiable.

## Conception

### 1. Le view model reprend le statut

```rust
// players/io/web/widgets/player_table_widget.rs
pub struct PlayerRowVm {
    …,
    /// `false` pour un joueur `MissingNextGame` — séquelle, blessure, absence.
    pub available: bool,
}
```

**Un booléen et non la chaîne du statut.** Le gabarit n'a qu'une question :
compte-t-il ou non. Lui passer `"MissingNextGame"` l'obligerait à connaître les
valeurs de l'énumération, et à se tromper le jour où une troisième apparaît.

**Aucune lecture supplémentaire** : la donnée arrive déjà jusqu'au widget.

### 2. Le sous-total, calculé au builder

```rust
pub struct PlayerTableTemplate {
    …,
    pub available_count: usize,
    pub unavailable_count: usize,
    pub available_value_kpo: i32,
}
```

Trois nombres plutôt qu'une structure : le gabarit les affiche, il ne les
manipule pas.

**`unavailable_count` sert à la mention « 1 absent, hors du compte ».** Elle
n'apparaît que s'il y en a — sinon la phrase serait un bruit permanent pour un
cas rare.

### 3. Le gabarit

Un `<tfoot>`, **pas une `<tr>` de fin de `<tbody>`** : le pied reste attaché au
tableau si la liste défile un jour, et il ne prend ni le zébrage
(`tr:nth-child(even)`) ni le survol des lignes de joueur.

Huit colonnes comme l'en-tête, dont un `colspan="6"` pour le libellé.

**La colonne SPP porte un tiret, pas un total.** Les SPP ne s'additionnent pas
entre joueurs — chacun dépense les siens — et une somme y serait un nombre sans
signification.

### 4. Le marquage de l'absent

```html
<tr class="player-row is-out">
  …<strong>Mirindel</strong><span class="out-tag">Absent au prochain match</span>
```

Ligne grisée, nom en graisse normale, pastille rouge pâle. **Discret mais
lisible** : l'absence est un état, pas une alerte.

### 5. CSS

Dans `assets/static/css/pages/team-page.css`, qui porte déjà `.player-table` et
est au bundle. **Aucune feuille neuve**, rien à inscrire dans `css_bundle.rs`.

## Ce que la carte ne fait pas

- **Elle ne scinde pas le tableau.** Les absents restent dans la liste, à leur
  place, simplement marqués et hors du compte.
- **Elle ne touche pas à la valeur d'équipe.** Le sous-total est un affichage ;
  `team_value.rs` garde son calcul, qui compte d'autres choses — staff,
  relances, journaliers déduits.
- **Elle n'explique pas l'absence.** Ni séquelle, ni blessure, ni durée : la
  fiche du joueur le dit.

## Un écart à surveiller

Le sous-total et la valeur d'équipe de l'en-tête **ne seront pas égaux**, et
c'est normal : la VE inclut le staff, les relances et la déduction des
journaliers.

Un coach pourrait s'en étonner. Le libellé « joueurs disponibles » le dit déjà —
c'est un sous-total de joueurs, pas la valeur d'équipe — mais c'est le genre
d'écart qui remonte en question, et il vaut mieux l'avoir prévu.

## Tests

| Test | Ce qu'il prouve |
|---|---|
| `le_sous_total_exclut_les_indisponibles` | le cœur |
| `sans_indisponible_la_mention_n_apparait_pas` | le cas le plus fréquent |
| `un_effectif_vide_ne_rend_pas_de_pied` | le tableau vide a déjà son message |
| `available_est_faux_pour_missing_next_game` | le VM |
| **E2E** : le pied affiche le compte et la somme justes | bout en bout |

## Checklist

- [ ] `available` sur `PlayerRowVm`
- [ ] Les trois nombres sur `PlayerTableTemplate`
- [ ] Le `<tfoot>`, huit colonnes, tiret sur SPP
- [ ] Le marquage `is-out` et la pastille
- [ ] Les styles dans `team-page.css`
- [ ] Les quatre tests unitaires et le test e2e
- [ ] `make lint && make test && make check-arch`
