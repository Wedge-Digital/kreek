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

## Le rejeu « sans l'événement visé » ne suffit pas

La carte disait : charger le flux, **en retirer l'événement visé**, rejouer, lire
la valeur. Cela ne tient qu'au **premier** retrait.

```
créé(100) · C1 prix +50 → 150 · C2 prix +30 → 180 · retrait de C2 pose 150
```

Retirons C1. Le flux privé du seul C1 vaut `[créé(100), C2(+30)=130,
retraitC2(pose 150)]` → **150**, c'est-à-dire exactement la valeur courante :
le retrait aurait été un **no-op silencieux**. La bonne réponse est 100.

La cause est que `value_after` est absolu — ce que la carte a raison d'exiger,
l'écrêtage n'étant pas inversible — mais un absolu **posé dans un monde où C1
existait encore**.

**Le rejeu porte donc sur le flux effectif** : privé de l'événement visé, de
toutes les customisations déjà retirées, et de tous les événements de retrait.
`domain/customisations.rs::flux_effectif`. La forme de l'événement n'a pas
changé ; seul le filtre.

Le test `deux_retraits_de_prix_successifs_partent_chacun_de_l_etat_reel` le
verrouille — écrit avec **deux customisations de prix**, sans quoi il serait
passé au vert sans rien prouver. La mutation qui rétablit la conception d'origine
fait tomber trois tests.

## Trois consommateurs, un seul filtre

`flux_effectif` sert le rejeu du use case, la liste du panneau, **et le journal**.
Un même endroit décide de ce qui tient encore, et deux conséquences en découlent
sans garde dédiée :

- un second retrait du même identifiant se refuse tout seul — la customisation
  n'est plus dans le flux effectif ;
- une customisation retirée **disparaît du journal**, son retrait avec elle.

Ce dernier point est une décision produit, prise en cours de carte : le journal
est une vue du **joueur**, pas de son dossier. Y laisser une customisation qui ne
s'applique plus, ou une ligne « retirée » répondant à une ligne encore lisible,
raconterait deux fois ce qui n'a plus lieu. L'event store, lui, garde les deux —
l'audit les y retrouve.

## Ce que la vérification à l'écran a corrigé

Le panneau annonçait « Le joueur perdra ces **+25** kPo de valeur ». Le « + » est
maladroit ; sur un ajustement **négatif** la phrase aurait dit « perdra ces −25
kPo », soit **le contraire de la vérité** — retirer une baisse de prix *rend* de
la valeur au joueur. La phrase suit maintenant le signe, et un test la verrouille.

Aucun test unitaire ne l'aurait vu : la maquette ne montrait que le cas positif,
et la formulation était juste pour lui.

## Sans date : l'auteur seul

La maquette affiche « BigBoss, 02/07/2026 ». `occurred_at` existe bien dans
`players_events`, mais `find_events_by_id` ne le rend pas, et l'exposer
demanderait d'élargir le port pour une colonne d'affichage. Décidé : **l'auteur
seul**.

## Le piège d'outillage rencontré

`cargo test` nu pointe sur la base de **développement** et fait échouer *tous* les
tests `sqlx::test`. Deux falsifications de projection ont d'abord semblé
concluantes alors qu'elles ne mesuraient rien. `make test` pose
`DATABASE_URL=$(TEST_DB_URL)` — c'est lui qui fait foi, et une falsification
lancée à la main doit reprendre la même variable.

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
