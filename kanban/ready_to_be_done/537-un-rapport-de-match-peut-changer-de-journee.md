# Un rapport de match peut changer de journée

**Priorité : haute — son absence a coûté cinq heures de saisie en production**
**Dépend de :** rien
**Épic :** E17 — Corriger le calendrier sans perdre la saisie
**Fichiers :** `src/app/match_report/domain/events.rs`,
`domain/match_report_{draft,pre_match,ready_to_publish,published}.rs`,
`domain/match_report_state.rs`,
`use_cases/change_round_use_case.rs` (nouveau)

## Ce qui l'a fait écrire

Le 7 septembre, dans l'espace G. B. L. R, un coach a saisi un match sur la
journée 1 alors qu'il se jouait à la journée 15. Faute de pouvoir le déplacer,
il a **annulé son rapport** et tout ressaisi ailleurs — puis un second geste
d'administration a annulé le second. Les deux rapports sont annulés, cinq heures
de saisie perdues à l'écran.

Le geste qui manquait tient en une phrase : *ce match n'est pas le bon jour.*

## Ce que la mesure a montré

`round_id` n'a que **deux usages** dans tout le BC `match_report` :

| Usage | Nature |
|---|---|
| `find_id_by_round_and_teams` | la clé de dédoublonnage — « un rapport existe-t-il déjà pour cette affiche ce jour-là » |
| `build_round_context_vm` | les libellés affichés |

**Aucune règle métier n'en dépend**, dans aucun état. Ce n'est pas une donnée
qui gouverne, c'est une donnée qu'on transporte — d'où une carte modeste là où
l'intuition annonçait un chantier.

## Conception

`round_id` n'est porté que par `MatchReportCreated`. Il faut donc un événement,
et sa prise en compte dans les quatre états — `Draft`, `PreMatch`,
`ReadyToPublish`, `Published` — où l'identifiant est recopié.

```rust
MatchReportDomainEvent::RoundChanged { round_id: RoundId, changed_by: CoachId }
```

**Le déplacement est permis dans les quatre états, publié compris.** Rien ne s'y
oppose : le classement ne lit jamais la journée d'un rapport (cf. carte 538), et
un match publié qui change de jour reste le même match. Distinguer les états
aurait produit une règle sans motif.

### La seule garde

La journée d'arrivée ne doit pas déjà porter la même affiche —
`find_id_by_round_and_teams` répond exactement à cette question. Sans elle, on
crée le doublon que la fonction sert à éviter, et c'est précisément le doublon
de l'incident.

Refus : `ChangeRoundError::FixtureAlreadyOnTargetRound`.

## Ce que cette carte ne fait pas

**Rien ne se voit encore.** Le calendrier, les résultats et l'écran viennent des
cartes 538 et 539. Livrée seule, celle-ci est un événement que personne n'émet —
c'est assumé : elle compile, elle se teste, et elle porte la décision.

## Checklist

- [ ] `RoundChanged` dans `MatchReportDomainEvent`
- [ ] La transition dans les **quatre** états, et sa réhydratation
- [ ] `ChangeRoundCommand`, avec `RoundId` et `CoachId` typés
- [ ] `change_round_use_case`, instrumenté, sa garde de doublon
- [ ] Tests : déplacement depuis chacun des quatre états · refus si l'affiche
      existe déjà sur la journée cible · le rapport garde tout le reste —
      actions, gains, joueurs temporaires
- [ ] `make lint`, `make check-arch`, `make test`
