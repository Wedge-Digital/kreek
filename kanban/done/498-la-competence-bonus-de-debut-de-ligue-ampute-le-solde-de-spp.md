# La compétence bonus de début de ligue ampute le solde de SPP du joueur

**Priorité : haute** — le joueur ne peut pas dépenser ce qu'il a gagné
**Contexte :** `players` · **Sans épic** · **Signalée par :** l'utilisateur

## Le symptôme

Un joueur prend Blocage sur les SPP bonus de début de ligue, puis gagne 8 SPP au
match suivant. Son solde affiche **2**, alors qu'il devrait afficher **8**.

## La cause

Les SPP bonus sont un **budget d'équipe**, pas du joueur : `spp_pool` sur
l'agrégat de `team_creation`, partagé entre tous ses joueurs
(`spp_budget_widget.rs:47`). Le joueur, lui, naît avec `starting_spp: Spp(0)`
(`player_creation.rs:104`) — et c'est correct, il n'a jamais possédé ces points.

Mais la compétence prise sur ce budget est enregistrée par `InitialSkillEarned`,
et l'agrégat l'empile dans `acquired_skills` **avec son coût réel**
(`player.rs:320`). Or le solde soustrait tous les coûts sans distinction :

```rust
pub fn spp_remaining(&self) -> u32 {
    let spent = Σ acquired_skills.spp_cost + Σ stat_increases.spp_cost;
    self.spp.0.saturating_sub(spent)
}
```

Le joueur est donc **débité d'une dépense qu'il n'a jamais faite** : `0 - 6`
saturé à 0, puis `8 - 6 = 2`.

Une fois dans l'agrégat, la compétence bonus est **indistinguable** d'un achat
normal — même `mode` (Chosen ou Random), même `from_match: None`. Seul
l'événement d'origine les sépare, et c'est déjà ce dont le journal se sert pour
afficher « Compétence initiale bonus ».

## Aucune migration à jouer

`spp_remaining()` est **dérivé au rejeu, jamais stocké** — le domaine le dit
lui-même. Corriger la fonction corrige tous les joueurs de production au
prochain affichage, sans toucher une ligne de données.

`players_proj.spp` porte le cumul des gains, pas le solde : il est juste, et le
reste.

Et **aucune donnée fausse n'a été écrite**. Les gardes d'achat testent
`spp_remaining() < coût` : avec un solde sous-estimé, elles ont **refusé des
achats légitimes** — jamais autorisé d'achat illégitime. Il n'y a rien à
réparer, seulement des coachs qui n'ont pas pu dépenser ce qu'ils avaient.

Le champ ajouté à `AcquiredSkill` porte `#[serde(default)]` : `acquired_skills`
est sérialisé en JSONB dans la projection, et les lignes existantes ne le
portent pas. Elles se liront donc « pas un bonus », ce qui est faux — mais **la
projection ne sert qu'à l'affichage des pastilles**. Le solde vient de
l'agrégat, rejoué depuis les événements, où l'information est intacte.

## Ce que la correction ne change pas

**Le niveau du joueur.** `est_une_amelioration()` se fonde sur le `mode`, qui
vaut Chosen ou Random pour une compétence bonus : elle continue de compter dans
le niveau, donc de renchérir la suivante. Validé avec le PO. C'est cohérent avec
la carte 482 — « seules les compétences qui ont coûté des SPP comptent », et
celle-ci en a coûté, même bonus.

**Les caractéristiques.** Le budget initial n'en achète pas — confirmé avec le
PO, et aucun événement `InitialStatIncrease` n'existe. `stat_increases` reste
soustrait tel quel.

**Les neuf appelants du solde.** Ils se corrigent d'un coup, `spp_remaining()`
étant le seul calcul du solde de toute l'application — vérifié, aucune autre
somme de `spp_cost` ailleurs. Y compris `player_customisation_widget:497`, qui
dérive les « dépenses » de `spp - spp_remaining()` et cessera donc de compter le
bonus comme une dépense du joueur.

## Tests

| Test | Ce qu'il prouve |
|---|---|
| le scénario signalé | compétence initiale à 6, puis 8 gagnés → solde **8** |
| le contrôle | achat **normal** à 6 dans les mêmes conditions → solde **2** |
| le mélange | les deux sur le même joueur, seul le second débite |

Le contrôle n'est pas décoratif : sans lui, la correction passerait aussi si elle
cessait de compter **toutes** les dépenses.

## Checklist

- [x] `sur_budget_initial` sur `AcquiredSkill`, posé à `true` dans la seule
      branche `InitialSkillEarned`, avec `#[serde(default)]`. **Sans valeur par
      défaut côté Rust** : le compilateur a listé les sept sites de
      construction, dont trois doublures de test. Un `..Default::default()` les
      aurait rangés en silence
- [x] `spp_remaining()` exclut ces coûts — une ligne, neuf appelants corrigés
- [x] **Quatre** tests unitaires : le scénario signalé, le contrôle sur un achat
      normal, le mélange des deux, et le niveau qui continue de monter
- [ ] **Test e2e : non fait, et volontairement.** Les 70 `InitialSkillEarned` de
      la base locale viennent de la production importée, pas des fixtures —
      `build_full_competition` n'en crée aucun (40 équipes sur 1007 en portent).
      Il faudrait donc une fixture neuve : compétition dotée d'un pool de SPP,
      équipe construite, compétence choisie, match joué. Disproportionné pour une
      règle tenue par quatre tests unitaires, et redondant avec l'e2e de la carte
      492, qui prouve déjà que l'écran affiche `spp_remaining()`. **À reprendre
      le jour où une fixture à budget initial existera pour d'autres raisons.**
- [x] `make lint`, `make check-arch` (17 axes), `make test` (1648),
      `make e2e` (**356 passés**, suite complète 67/67, 0 échec)

## Terminé quand

Un joueur ayant pris une compétence sur le budget initial, puis gagné 8 SPP,
affiche 8 sur sa fiche comme dans la liste de son équipe — et peut les dépenser.
