# Le journalier a sa place, et le style de la maison

**Priorité : haute — deux écrans livrés qui trompent le coach**
**Épic :** E15 — Recruter un journalier (suites)
**Dépend de :** 454 à 459 et 502, toutes livrées
**Fichiers :** `src/app/match_report/io/web/widgets/temp_player_selector_widget.rs`,
`src/app/match_report/io/web/templates/temp-player-selector-widget.html`,
`src/app/players/io/web/templates/journeymen-widget.html`,
`src/app/players/io/web/widgets/journeymen_widget.rs`,
`assets/static/css/widgets/rec-page.css`
**Maquette :** `assets/rawpages/html/app-team-recruitment.html`

## Deux défauts signalés, un même oubli

L'épic a fait du journalier un joueur. **Les deux écrans qui l'affichaient
avant ne l'ont pas appris** : l'un le montre encore comme un remplaçant, l'autre
lui a inventé un style au lieu de reprendre celui de la maison.

## 1. Il apparaît deux fois à la saisie des actions

Depuis que la carte 454 a ouvert `find_alive_by_team_id`, le journalier figure
dans les **joueurs réguliers**. Il figure aussi dans la section « Journaliers »
du sélecteur temporaire, qui vient du rapport. Le coach voit deux entrées pour
un seul homme.

### Mais la section ne disparaît pas — et c'est le point

```rust
match &tp.kind {
    TempPlayerKind::StarPlayer { .. } => stars.push(vm),
    _ => journeymen.push(vm),        // ← les mercenaires aussi
}
```

Le `_` ramasse **journaliers et mercenaires**. Un mercenaire n'existe pas dans
`players` et n'y existera jamais : il doit rester dans le sélecteur temporaire.

Il est donc aujourd'hui affiché sous un titre « Journaliers », avec un badge
« J » — **un défaut d'étiquetage antérieur à l'épic**, que le signalement met au
jour. Retirer les journaliers sans nommer les mercenaires laisserait une section
« Journaliers » qui n'en contient aucun.

### Ce qui change

| | |
|---|---|
| `render_temp_players` | trois groupes — vedettes, **mercenaires**, et plus de journaliers |
| Le gabarit | une section « Mercenaires », badge « M » |
| Le widget | disparaît quand les deux groupes sont vides — c'est déjà son comportement |

**Rien ne change à l'enregistrement.** Un journalier choisi dans la liste
régulière produit un `ActionPlayer::Regular`, le chemin direct. La résolution
d'un `ActionPlayer::Temp` en journalier (carte 502) **reste** : un rapport en
cours peut porter des actions déjà saisies par l'ancien chemin, et les perdre
serait pire que le doublon qu'on corrige.

## 2. Le panneau de recrutement a un style à lui

La maquette `app-team-recruitment.html` **contient déjà ce panneau**, et la
carte 458 la citait en référence. Elle prescrit :

```html
<div class="panel panel--jm">          <!-- le même panneau que les autres -->
  <div class="jm-warn">…</div>          <!-- le seul élément qui lui est propre -->
  <table class="buy-table">             <!-- LA MÊME table que les achats -->
```

et par ligne : `.price` pour le montant, `.price-note` pour la décomposition,
`is-blocked` pour un refus, `.act-btn` pour le bouton — **exactement ce que les
postes utilisent**.

**L'implémentation a inventé quinze classes `.rec-journeyman-*`** au lieu de
reprendre celles-là, `.price-note` comprise, qui existait déjà pour décomposer
un prix.

C'est la divergence que le `CLAUDE.md` documente à propos du format des kPo :

> Une mise en forme inventée pour un seul écran est une divergence, pas un
> raffinement — et c'est le seul écran qui se trompait.

### Ce qui change

Le panneau devient un `.panel` comme les autres, sa table une `.buy-table`, et
il ne garde en propre que **ce qui le distingue vraiment** : le liseré orange et
l'avertissement de perte. Les quinze règles CSS tombent ; il en reste trois.

**Il n'y a donc pas de maquette à faire** — il y a une implémentation à ramener
sur la maquette qui existait déjà.

## Ce que la carte ne fait pas

- Elle ne touche pas au sélecteur de joueurs réguliers, qui est correct.
- Elle ne change aucune règle : ni recrutement, ni SPP, ni plafond.
- Elle ne renomme pas `mr-player-chip--journeyman` en CSS tant que la classe
  sert encore aux mercenaires — le nom suivra le markup, dans le même geste.

## Tests

| Test | Ce qu'il prouve |
|---|---|
| `un_journalier_ne_figure_pas_chez_les_remplacants` | le doublon levé |
| `un_mercenaire_reste_chez_les_remplacants` | et ce qu'on ne casse pas |
| `un_mercenaire_n_est_plus_etiquete_journalier` | le défaut d'étiquetage |
| `test_le_journalier_n_apparait_qu_une_fois` (e2e) | à l'écran, bout en bout |
| `test_le_panneau_reprend_le_style_des_achats` (e2e) | `.buy-table` et `.act-btn` présents |

Le dernier vise les classes de la maison plutôt que l'apparence : un test de
navigateur ne juge pas un style, il constate qu'on a repris celui qui existe.

## Checklist

- [x] Trois groupes dans `render_temp_players`, journaliers exclus
- [x] Section « Mercenaires » au gabarit, badge « M »
- [x] Le panneau de recrutement en `.panel` / `.buy-table` / `.act-btn`
- [x] `.price-note` réutilisée pour la décomposition
- [x] **Quinze règles ramenées à cinq** dans `rec-page.css`
- [x] Sept tests — cinq prévus, deux de plus
- [x] `make lint`, `make check-arch`, `make test` — 1715 tests
- [x] `make e2e` — 366 passés, 7 ignorés

## Ce qui a été fait

### Les sélecteurs n'existent qu'en avant-match

`test_le_journalier_n_apparait_qu_une_fois_a_la_saisie` visait d'abord le
rapport du fixture — **publié**. Sur un rapport publié, `render_temp_players`
rend un `404` et la page ne compose aucun sélecteur : le test cherchait des
puces sur un écran qui n'en a pas.

Un fixture s'arrête donc aux coups de pouce, le moment exact où le journalier
vient de naître et où les deux sélecteurs sont vivants.

### Le test de style rejoint celui du panneau

Il lui fallait une équipe en phase de recrutement, et le scénario d'embauche
ferme cette phase avant lui. Plutôt qu'un troisième parcours à deux matchs pour
observer le même écran au même moment, les assertions de style ont rejoint
`test_le_panneau_montre_le_journalier`.

### Trois de mes propres tests visaient les classes que je retirais

Attendu : ils avaient été écrits contre le style maison de la carte 458. Ils
visent désormais `.panel--jm`, `.buy-table`, `.price` et `.act-btn` — et l'un
d'eux vérifie qu'**aucune** classe `rec-journeyman` ne subsiste, pour que la
divergence ne revienne pas par la petite porte.
