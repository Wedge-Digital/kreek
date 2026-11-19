> **Carte annulée le 2026-08-25 — fusionnée dans la [398](../ready_to_be_done/398-l-onglet-de-customisation-survit-a-l-enregistrement.md).**
>
> Les deux cartes visaient le même écran et l'on croyait qu'elles partageaient
> une cause. La mesure a montré l'inverse : **elles étaient en tension**. Ce qui
> empêche le défaut de la 327 — le `HX-Refresh` de l'enregistrement — est
> exactement ce que la 326 proposait de supprimer. Les corriger séparément
> aurait fait que l'une défasse l'autre.
>
> Le contenu ci-dessous est conservé tel qu'il était ; la 398 porte le
> diagnostic et la correction.

# `players` — La somme de SPP disponibles ne suit pas la customisation

**Priorité : haute** — bug de livraison du mode de customisation
**Dépend de :** rien, mais à traiter **avec** la carte 326 — même écran, cause
possiblement commune
**Fichiers :** `src/app/players/io/web/player_detail_controller.rs`,
`src/app/players/io/web/templates/player-detail.html`,
`src/app/players/io/web/widgets/player_customisation_widget.rs`

## Le problème

Sur la fiche de détail du joueur, la somme de SPP disponibles n'est pas mise à
jour après une customisation de SPP. L'utilisateur voit l'ancienne valeur alors
que la modification a bien été enregistrée.

Quatre endroits affichent ce chiffre, et la carte doit établir lequel ment :

| Emplacement | Affichage |
|---|---|
| `player-detail.html:66` | « SPP dépensés / gagnés » |
| `player-detail.html:73` | « X SPP en réserve » |
| `player-customisation-widget.html:21` | « SPP en réserve » du panneau |
| `evolution-journal-widget.html:5` | « SPP en réserve » du journal |

## Ce qui a déjà été vérifié — le chemin d'écriture est sain

Inutile de recommencer ces deux vérifications :

- **L'agrégat applique bien l'événement.** `Player::apply` sur
  `PlayerSppCustomised` ajoute le montant à `player.spp`
  (`domain/player.rs`, branche `PlayerSppCustomised`).
- **La projection aussi.** `player_repository.rs` fait
  `UPDATE players_proj SET spp = spp + $2` dans la transaction de l'append,
  conformément à la règle des projections.

Le montant est un `SppAmount(u8)` — pas de piège de signe entre le `as u32` de
l'agrégat et le `as i32` de la projection.

**Donc l'investigation commence côté lecture / affichage.**

## Premier pas — la question qui découpe le problème en deux

Reproduire, puis **recharger la page à la main (F5)** :

- La valeur devient correcte → c'est un problème de rafraîchissement du
  fragment, et la carte 326 tient probablement la même racine : le panneau ou
  la fiche n'est pas re-rendu, ou l'est avant que la validation ne soit
  visible.
- La valeur reste fausse → c'est le calcul ou la source lue. Regarder
  `compute_spp_breakdown` (`player_detail_controller.rs:353`), qui dérive la
  réserve de `player.spp_remaining()`, et `reserve_effective`
  (`player_customisation_widget.rs:349`), qui retranche en plus les dépenses
  déjà au panier. Ces deux calculs de « réserve » coexistent et peuvent
  diverger.

Trancher cette question **avant** de proposer un correctif : les deux branches
ne mènent pas au même fichier.

## Checklist

- [ ] La branche du diagnostic est écrite dans la carte avant correction
- [ ] Après une customisation de SPP, les quatre affichages du tableau
      ci-dessus montrent la même valeur, et la bonne
- [ ] Le cas du panier non vide est couvert : la réserve affichée tient compte
      des lignes en attente sans les compter deux fois une fois validées
- [ ] Test unitaire sur le calcul de réserve concerné (celui qui s'est révélé
      fautif, ou les deux s'ils divergent)
- [ ] Test e2e : ajouter des SPP par customisation, enregistrer, vérifier le
      chiffre affiché sur la fiche
- [ ] `make test` passe
- [ ] `make check-arch` passe
