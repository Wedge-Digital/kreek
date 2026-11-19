# La Haine atteint le joueur

**Priorité : haute**
**Dépend de :** 401
**Conception :** `docs/specs/haine/saisie-des-actions/{06-domaine,07-integration}.md`
**Fichiers :** `src/app/shared_kernel/app_events/player_match_impact_app_events.rs`,
`src/app/match_report/io/app_events/app_event_publisher.rs`,
`src/app/players/domain/player.rs`, `domain/events.rs`,
`io/repository/player_repository.rs`,
`io/app_events/player_match_impact_listener.rs`

## Objectif

À la publication du rapport, le joueur blessé porte sa Haine parmi ses
compétences acquises. À la dépublication, elle s'en va avec le reste.

## Le trajet

```
publication  →  PlayerInjured { context, injury_type, hatred_skill_uid }
             →  player_match_impact_listener
             →  player.record_hatred(context, skill_id, nom)
             →  acquired_skills += la compétence désignée
```

**On ne crée pas de second chemin.** Le mécanisme d'impact de match existe,
s'applique à la publication et sait se défaire : `TeamMatchImpactReverted` annule
l'impact sur tout l'effectif, la Haine comprise, **sans traitement particulier**.
Une écriture immédiate à la saisie aurait laissé la Haine derrière elle après une
dépublication.

Corollaire déjà acquis : **supprimer l'action supprime la Haine**, puisque avant
publication rien n'est parti.

## La règle du journalier est déjà tenue

`build_player_impact_events` (`app_event_publisher.rs:318`) filtre depuis
toujours :

```rust
let ActionPlayer::Regular(player_id) = &a.player else {
    return None; // BR1 — stars/mercenaires/journaliers exclus
};
```

La Haine d'un journalier reste donc dans le rapport de match — visible au
récapitulatif, absente de `players` — **sans une ligne écrite pour ça**. Rien à
ajouter ; simplement, ne pas défaire ce filtre.

## Le publisher recopie, il ne résout rien

**L'app event porte l'uid de la compétence, pas celui du mot-clef.**

```rust
PlayerInjured {
    context: PlayerMatchContextPayload,
    injury_type: InjuryTypePayload,
    #[serde(default)]
    hatred_skill_uid: Option<String>,   // « HAINE_BEASTMAN »
}
```

**Aucune traduction n'a lieu ici.** L'action porte déjà `hatred_skill_uid`,
figé au moment de la saisie par le use case de la carte 401, qui tenait le DTO du
mot-clef en main. Le publisher se contente de le recopier dans l'app event.

**Aucun port supplémentaire, donc**, et surtout aucune convention de nommage : la
première conception fabriquait `format!("HAINE_{uid}")`, le corpus porte
désormais le lien et la carte 399 le vérifie au démarrage. Un lien supposé est
devenu un lien déclaré, puis figé.

Le domaine de `match_report` garde le **mot-clef** à côté — c'est ce que le coach
a choisi, et ce que le récapitulatif du match affiche.

« Haine : Homme-Bête » reste résolu **par le listener**, via `ISkillCatalogPort`
que `players` possède déjà : un libellé change avec le corpus, un uid non, et
inscrire le nom dans l'event store y figerait une traduction.

## Le trait est gratuit, et l'état porte des zéros

```rust
AcquiredSkill {
    skill_id: SkillId::try_new(hatred_skill_uid)?,   // porté par l'app event
    skill_name,
    mode: AcquisitionMode::Injury,
    // Ni coût ni valeur : un trait gagné en encaissant un coup ne se paie pas
    // et ne renchérit pas le joueur.
    spp_cost: SppCost::try_new(0).unwrap(),
    value_delta: ValueKpo(0),
}
```

Le précédent exact est à `player.rs:523` — une compétence customisée y entre
ainsi. La distinction tenue par le projet : **l'événement** ne porte pas de champ
de valeur (« il n'existe pas, il ne vaut pas zéro »), **l'état projeté** porte
des zéros.

`AcquisitionMode::Injury`, et non `Automatic` : le coach répond puis choisit
parmi trente-huit mots-clefs — c'est le geste le moins automatique de l'écran.
Les trois modes existants nomment la façon d'obtenir ; la quatrième case est « à
la suite d'une blessure ». Le journal des évolutions affichera « Blessure »,
comme « Choisie » traduit `Chosen`.

## L'affichage est gratuit

La Haine entrant dans `acquired_skills`, la fiche joueur la rend déjà. Le badge
violet des traits et les mots-clefs du poste relèvent des **autres pages** de
cette fonctionnalité, qui n'ont pas encore leurs phases 2 à 7.

## Checklist

- [ ] `PlayerInjured` gagne `hatred_skill_uid: Option<String>`
- [ ] Le publisher **recopie** `hatred_skill_uid` depuis l'action ; aucun port
      nouveau, **aucun `format!` de convention nulle part**
- [ ] `AcquisitionMode::Injury` + branche de projection
- [ ] Événement domaine de gain, **sans champ de valeur**
- [ ] `Player::record_hatred` — pas de `Result` : tout est vérifié en amont, et
      une méthode qui ne peut pas échouer ferait écrire des `unwrap`
- [ ] Le listener résout le libellé au catalogue
- [ ] Tests unitaires :
  - [ ] gain → `spp_cost` 0, `value_delta` 0, mode `Injury`
  - [ ] la réserve de SPP du joueur est **inchangée**
  - [ ] deux fois le même mot-clef → accepté (aucune règle de doublon)
  - [ ] trois Haines différentes → cumulées
  - [ ] dépublication → la Haine est défaite avec l'impact du match
  - [ ] action d'un journalier porteur d'une Haine → **aucun app event**
  - [ ] `PlayerInjured` sans `hatred_skill_uid` → comportement d'avant, inchangé
  - [ ] une action portant un `hatred_skill_uid` inconnu du catalogue au moment
        de la publication → **aucune compétence créée**, une ligne `warn` : la
        garde du démarrage rend le cas improbable, mais un corpus amputé entre
        la saisie et la publication ne doit pas produire une compétence vide
- [ ] `make lint`, `make check-arch`, `make test`
