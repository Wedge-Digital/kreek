# Solitaire est une customisation, pas une amélioration

**Priorité : haute — un cadeau facturé au joueur**
**Épic :** E15 — Recruter un journalier (suites)
**Dépend de :** 504, qui a choisi le mauvais mode
**Fichiers :** `src/app/players/io/app_events/player_creation.rs`,
`src/app/players/domain/player.rs`

## Le défaut

La carte 504 a donné Solitaire (4+) au journalier en `AcquisitionMode::Chosen`,
avec ce commentaire :

> Ni choisie ni tirée : le règlement la donne. `Chosen` est le mode le moins
> faux des quatre.

**Il était faux.** Le mode ne décide pas que d'un libellé :

```rust
pub fn est_une_amelioration(self) -> bool {
    match self {
        Self::Chosen | Self::Random => true,
        Self::Customised | Self::Injury => false,
    }
}
```

`next_improvement_level` compte les améliorations pour fixer **le prix de la
compétence suivante**. Avec `Chosen`, Solitaire compte : un journalier recruté
paie sa première vraie compétence **un niveau plus cher**, pour un trait que le
règlement lui a donné.

C'est le défaut que la carte 482 avait corrigé ailleurs, et son commentaire le
dit déjà :

> Les compter renchérirait l'amélioration d'après, ce qui revient à **faire
> payer un cadeau**.

### Le `match` exhaustif avait posé la question

Il est sans joker, et son commentaire explique pourquoi : *« un cinquième mode
doit casser la compilation et forcer son auteur à trancher »*. Il ne pouvait pas
m'arrêter — je n'ajoutais pas de mode, j'en choisissais un — mais il nommait la
conséquence à deux lignes de mon choix. Je l'ai lue sans la voir.

## Ce que `Customised` apporte

| | |
|---|---|
| Prix de la suivante | ne compte plus — le cadeau cesse d'être facturé |
| Journal d'évolution | pastille « Customisation » au lieu de « Choisie » |
| Sémantique | ce que le joueur porte sans l'avoir payé |

## Le commentaire du mode doit s'élargir

Aujourd'hui :

> Donnée par un commissaire, hors des règles du jeu.

Solitaire vient **du règlement**, pas d'un commissaire. Laisser ce commentaire
en l'état créerait la contradiction suivante : le mode dirait une chose, son
usage principal en dirait une autre.

Ce que les quatre modes partagent réellement est plus simple — `Customised` et
`Injury` désignent **ce que le joueur n'a pas payé de ses SPP**, et c'est
exactement ce que `est_une_amelioration` distingue.

## Ce que la carte ne fait pas

- Elle ne touche pas à `est_une_amelioration` : la règle est bonne, c'est son
  entrée qui était mauvaise.
- Elle ne renomme pas `Customised`. Le nom couvre mal le cas du règlement, mais
  un renommage traverserait l'event store — les modes sont persistés — pour un
  gain de vocabulaire. Le commentaire élargi suffit.

## Tests

| Test | Ce qu'il prouve |
|---|---|
| **`solitaire_ne_reencherit_pas_la_competence_suivante`** | **le vrai défaut** — niveau 1, pas 2 |
| `solitaire_est_pose_en_customisation` | le mode, à la naissance |
| `test_solitaire_porte_la_pastille_de_customisation` (e2e) | ce que le coach voit |

Le premier est celui qui compte. Les deux autres sont de l'affichage ;
celui-là est de l'argent — et aucun test ne le couvrait, ni en 502 ni en 504.

## Checklist

- [x] `AcquisitionMode::Customised` à la naissance du journalier
- [x] Le commentaire du mode élargi au règlement
- [x] Les trois tests
- [x] `make lint && make test && make check-arch && make e2e`

`make test` : 1718 passés. `make e2e` : 368 passés, 7 sautés — dont les douze
du journalier, dernier arrivé compris.

## Ce qui a été fait

`solitaire_du_journalier` pose désormais `AcquisitionMode::Customised`, avec le
commentaire qui dit **pourquoi le mode compte** : il ne choisit pas un libellé,
il décide si le trait fait monter le joueur d'un niveau — donc le prix de sa
compétence suivante.

Le commentaire de `AcquisitionMode::Customised` a été élargi : il ne parle plus
du seul commissaire, mais de ce que le joueur porte **sans l'avoir payé de ses
SPP** — ce que `est_une_amelioration` distingue réellement.

### Les trois tests

| Test | Où |
|---|---|
| `solitaire_ne_reencherit_pas_la_competence_suivante` | `domain/player.rs` |
| `solitaire_est_pose_en_customisation` | `io/app_events/player_creation.rs` |
| `test_solitaire_porte_la_pastille_de_customisation` | `tests/e2e/test_journeyman_recruitment.py` |

**Le premier a été vu mordre.** Le mode remis à `Chosen`, il rend `left: 2,
right: 1` : le journalier montait bien d'un niveau, et payait sa première vraie
compétence au tarif du second. C'était la seule façon de prouver que le test
couvre le défaut plutôt que le correctif.

Il porte une contre-épreuve : une vraie compétence acquise, elle, fait bien
passer le joueur au niveau 2. Sans elle, un test qui répondrait toujours 1 —
parce que `next_improvement_level` serait cassé — passerait aussi.

Le second construit le trait avec un catalogue **muet** : le libellé retombe
alors sur son repli, et rien dans le test ne dépend du référentiel.

Le troisième lit la pastille du journal d'évolution, seule trace visible du
mode. Il ne remplace pas le premier : l'un couvre l'affichage, l'autre l'argent.

### Ce que le `match` exhaustif n'a pas pu faire

Il est sans joker, et son commentaire nommait la conséquence à deux lignes du
choix fautif. Il ne pouvait pas m'arrêter : je n'ajoutais pas un cinquième mode,
j'en choisissais un parmi quatre. **Un `match` exhaustif verrouille l'ajout d'un
cas, jamais le mauvais choix parmi les cas existants** — c'est un test qui le
fait, et c'est celui qui manquait en 502 comme en 504.
