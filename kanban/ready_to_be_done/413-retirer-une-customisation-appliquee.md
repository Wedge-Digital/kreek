# Retirer une customisation appliquée

**Priorité : moyenne**
**Dépend de :** rien
**Maquette :** `assets/rawpages/html/app-player-detail-custo-suppression.html`
**Fichiers :** `src/app/players/domain/{events.rs, player.rs}`,
`use_cases/revert_customisation_use_case.rs` (nouveau),
`io/web/widgets/evolution_journal_widget.rs`,
`io/web/widgets/player_customisation_widget.rs`,
`io/repository/player_repository.rs`, `routes.rs`

## Objectif

En mode customisation, un commissaire peut **retirer une customisation déjà
appliquée** au joueur, et l'effet est défait.

Aujourd'hui le mode ne sait qu'ajouter. Une erreur de saisie — un prix ajusté du
mauvais montant, une compétence offerte au mauvais joueur — ne se corrige que par
une seconde customisation qui compense la première, et le journal garde les deux.

## Ce qui existe déjà, et qu'on ne recrée pas

**Les quatre événements portent un `customisation_id`** —
`PlayerSkillCustomised`, `PlayerStatCustomised`, `PlayerValueCustomised`,
`PlayerSppCustomised`. Le retrait peut donc viser **une ligne précise**, et non
un lot : `validate_customisation_use_case` attribue un identifiant **par ligne**,
pas par panier (`zip(cmd.customisation_ids.iter())`).

**Le journal lit déjà l'event store** : `evolution_journal_widget` construit ses
lignes depuis `&[PlayerDomainEvent]`. Lister les customisations d'un joueur ne
demande donc **aucune projection nouvelle** — un filtre sur les quatre types, et
l'exposition du `customisation_id`.

## Un seul événement de compensation

```rust
PlayerCustomisationReverted {
    player_id: PlayerId,
    team_id: TeamId,
    customisation_id: CustomisationId,   // celle qui est défaite
    undo: UndoEffect,
    author: String,                      // arch:ok
}

pub enum UndoEffect {
    Skill { skill_id: SkillId },
    Stat  { stat: StatKind, offset: i8 },
    Value { value_after: ValueKpo },     // ← absolue, voir plus bas
    Spp   { amount: SppAmount },
}
```

**Un fait, quatre formes.** Le nom dit ce qui s'est passé — une customisation a
été retirée — et l'enum dit ce que ça défait. Quatre événements distincts
diraient quatre fois le même fait.

**L'événement porte ce qu'il faut pour défaire**, parce que `apply()` traite les
événements un par un, sans accès au reste du flux : il ne peut pas retrouver la
customisation d'origine pour en déduire l'inverse.

## La valeur est absolue, jamais un delta inverse

`apply` fait `value = max(value + delta, 0)`. Un joueur à 30 kPo qui subit un
−50 tombe à **0**, pas à −20 : ajouter +50 pour défaire lui donnerait 50, soit 20
de plus qu'avant. **L'écrêtage n'est pas inversible.**

Le projet connaît déjà ce piège et sa solution — `PostMatchSequenceReverted` :

> *`dedicated_fans` est la valeur **absolue** restaurée, pas un delta : l'écrêtage
> à 0..20 n'est pas inversible.*

**Comment obtenir cette valeur** : le use case charge le flux du joueur, en
**retire l'événement visé**, rejoue par `Player::from_events`, et lit la valeur
obtenue. C'est exact par construction — c'est le joueur tel qu'il serait si la
customisation n'avait jamais eu lieu — et ça ne demande aucun calcul inverse.

## Le refus, quand les SPP ont été dépensés

Un commissaire offre 10 SPP, le coach en dépense 12. Retirer la customisation
mettrait le joueur à −2.

**La suppression est refusée** si `spp_remaining()` est inférieur au montant
offert. Pas d'écrêtage silencieux, pas de cascade sur les améliorations déjà
payées : le commissaire voit pourquoi et décide.

L'écran l'annonce **avant** le clic — croix désactivée, motif écrit dessous
(« ces SPP ont été dépensés — il n'en reste que 3 sur 10 »). Une croix qui
disparaît laisse croire à un bug ; une croix grise qui s'explique enseigne la
règle. Le domaine refuse quand même, l'écran n'étant pas une garantie.

**Les trois autres types ne se refusent jamais** : retirer une compétence, une
caractéristique ou un ajustement de prix ne peut mettre le joueur dans aucun état
impossible.

## L'effet de bord à réémettre

Seule la customisation de **prix** franchit la frontière du BC —
`PlayersAppEvent::PlayerValueCustomised`, qui déclenche le recalcul de la valeur
d'équipe. Son retrait doit donc **réémettre le même app event** : celui-ci ne
porte que l'équipe et le joueur, et provoque un recalcul complet. Rien à
inventer.

`to_app_event()` gagne un bras pour `PlayerCustomisationReverted` **avec
`UndoEffect::Value` uniquement**. Les trois autres restent dans le BC, comme
leurs customisations d'origine.

Attention au joker : `to_app_event` finit par `_ => None`, avec ce commentaire —
« le compilateur ne signalera pas un événement qu'on oublierait de faire sortir
du BC. Ajouter un bras est délibéré ». C'est un ajout délibéré.

## L'écran

La section **« Customisations appliquées »** rejoint le panneau de customisation,
au-dessus de « Modifications en attente ». Une croix par ligne, une confirmation
**qui s'ouvre sous la ligne** — pas de modale : ce qu'on défait reste sous les
yeux pendant qu'on décide.

Chaque confirmation dit son effet dans les termes du joueur, et ils diffèrent :

| Ligne | Ce que la confirmation annonce |
|---|---|
| Prix | « perdra 7 000 Po de valeur, et la valeur d'équipe sera recalculée » |
| Compétence | « perdra la compétence Bloc. Aucun SPP n'est rendu : elle n'en avait pas coûté » |
| Caractéristique | « repassera à AG 4+. Sa valeur ne bouge pas » |

**Seules les customisations sont listées** — les améliorations payées en SPP
n'apparaissent pas : elles ne se défont pas ici.

## Le droit

Celui du mode customisation : `can_customise` — **admin d'espace ou admin de
compétition, jamais le coach**. Le POST est gardé comme les autres actions de
customisation.

## Checklist

- [ ] `PlayerCustomisationReverted` + `UndoEffect` dans `events.rs`
- [ ] `apply()` : retrait de `acquired_skills`, de `stat_customisations`,
      valeur **absolue** posée, SPP soustraits
- [ ] Branche de projection dans `player_repository`
- [ ] `Player::revert_customisation(...)` — refuse le cas SPP dépensés
- [ ] `revert_customisation_use_case` : rejeu sans l'événement pour obtenir la
      valeur absolue
- [ ] `to_app_event()` : un bras pour `UndoEffect::Value` seulement
- [ ] Le journal expose le `customisation_id` et le type de chaque customisation
- [ ] Section « Customisations appliquées » dans le widget de customisation,
      confirmation inline, croix désactivée avec motif
- [ ] Route et handler du retrait, gardés par `can_customise`
- [ ] Tests unitaires :
  - [ ] retrait d'une compétence → elle quitte `acquired_skills`, SPP inchangés
  - [ ] retrait d'une caractéristique → l'offset disparaît, valeur inchangée
  - [ ] **retrait d'un prix après écrêtage** : 30 kPo, −50 appliqué (donc 0),
        retrait → **30 kPo**, et non 50
  - [ ] retrait de SPP avec réserve suffisante → soustraits
  - [ ] retrait de SPP **dépensés** → refusé, aucun événement
  - [ ] retrait d'un `customisation_id` inconnu → refusé
  - [ ] deux retraits successifs → chacun part de l'état courant
  - [ ] seul le retrait d'un prix produit un app event
- [ ] Test e2e : poser une customisation de prix, la retirer, vérifier la valeur
      du joueur **et** celle de l'équipe
- [ ] `make lint`, `make check-arch`, `make test`
