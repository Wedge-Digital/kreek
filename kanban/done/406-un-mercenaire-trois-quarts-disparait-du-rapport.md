# Un mercenaire trois-quarts disparaît du rapport

> **⚠️ Défaut reproductible en production, et silencieux.** Le mercenaire est
> proposé, facturé, accepté par le serveur — puis il n'existe nulle part. Aucune
> erreur, aucune ligne de journal.

**Priorité : haute** — reproductible à chaque fois, sur le poste le plus courant
**Dépend de :** rien
**Trouvé le :** 2026-08-25, en cherchant pourquoi un mercenaire n'apparaissait
plus dans la liste des joueurs pouvant agir

## Le symptôme

Engager un mercenaire **trois-quarts** dans un rapport de match. Il est déduit du
budget de coups de pouce, le POST répond sans erreur — et aux étapes suivantes,
il n'apparaît pas parmi les joueurs pouvant faire des actions.

Avec n'importe quel autre poste — percuteur, receveur, lanceur — tout fonctionne.

## La chaîne exacte

**1. `InducementQty` est borné à 10** (`domain/value_objects.rs:82`) :

```rust
#[nutype(validate(greater_or_equal = 1, less_or_equal = 10))]
pub struct InducementQty(u8);
```

**2. La spec du mercenaire porte le `max_qty` du poste**, pas la quantité achetée
(`record_inducements_use_case.rs:344`) :

```rust
for (uid, (cost, qty, max_qty_for_pos)) in groups {
    let induction_id = InducementId(uid);
    if let (Ok(mq), Ok(uc)) = (
        InducementQty::try_new(max_qty_for_pos),   // ← 16 pour un trois-quarts
        InducementCost::try_new(cost),
    ) {
        specs.push(AllowedInducementSpec { … });    // ← jamais atteint
    }
    tuples.push((induction_id, qty));               // ← poussé quand même
}
```

Le `try_new(16)` échoue, la spec n'est **pas** poussée — mais le tuple l'est,
`tuples.push` étant **hors du `if let`**. Un achat orphelin, sans spec.

**3. Le domaine écarte l'orphelin en silence** (`match_report_pre_match.rs:547`) :

```rust
purchases.iter().filter_map(|(uid, qty)| {
    allowed_specs.iter().find(|s| &s.uid == uid).map(|spec| InducementPurchase { … })
})
```

Pas de spec, pas d'achat. `InducementsRecorded` est bien émis, mais **sans le
mercenaire** : il n'entre jamais dans `home_inducements` / `away_inducements`.

**4. Plus rien en aval ne peut le retrouver.** `collect_mercs`
(`init_temp_players_use_case.rs:113`) lit `purchases_for(team_id)` et filtre les
uids `MERCO:` — il ne trouve rien, aucun `TempPlayer::Mercenary` n'est créé, et
le sélecteur de joueurs temporaires n'a rien à afficher.

## Pourquoi les trois-quarts, et eux seuls

C'est le seul poste dont `max_quantity` dépasse 10 :

| Poste | `max_quantity` |
|---|---|
| **Trois-quarts / piétaille** | **16** |
| Percuteur, receveur | 4 |
| Lanceur, rôdeur, mutant | 2 |
| Colosse | 1 |

**La corrélation avec `is_journeyman` est fortuite** : c'est `max_quantity` qui
décide, pas le drapeau. Un poste non journalier à douze exemplaires produirait
exactement le même défaut. Il se trouve que le lineman est le poste sans limite
pratique, d'où 16, et qu'il est aussi celui qu'on désigne journalier.

## Deux défauts, pas un

**Le mauvais champ dans le mauvais type.** `InducementQty` borne une *quantité
achetée* — dix coups de pouce identiques, c'est déjà généreux. On lui fait porter
un *plafond de roster*, qui vit sur une autre échelle. Ce sont deux notions, et
le type ne peut pas servir les deux.

Aggravant : `validate_mercenary_limit` plafonne **déjà** les mercenaires à trois.
Le `max_qty` de la spec ne protège donc rien sur ce chemin — il ne fait que
disqualifier des postes légitimes.

**L'échec avalé, deux fois de suite.** Le `if let` sans `else` laisse un tuple
sans sa spec ; le `filter_map` du domaine le jette sans un mot. Ni 422, ni
`warn`. C'est le cinquième cas de la liste que tient le `CLAUDE.md` —
`UnknownSkill` accusant le catalogue, le poste replié sur « Joueur », le roster
escamoté deux fois par un `.ok()?`. Et c'est le plus coûteux des cinq : le coach
a **payé**.

## Ce que la correction doit faire

**Séparer les deux notions.** Le plafond porté par `AllowedInducementSpec` n'est
pas une quantité d'achat. Soit un type propre, soit un `u8` nu documenté — ce
champ ne protège aucun invariant que `validate_max_qty` ne vérifie déjà.

**Rendre l'orphelin impossible.** Un tuple ne doit pas pouvoir survivre à
l'échec de sa spec : construire les deux ensemble, ou ne rien pousser.

**Rendre l'écart bruyant.** `build_purchase_list` doit refuser plutôt que
filtrer : un achat sans spec est une incohérence d'appelant, pas une donnée à
ignorer. Une `DomainError` remonte en 422 et laisse une trace — le domaine ne
journalise pas, il refuse.

**Ne pas élargir `InducementQty`.** Passer sa borne à 16 ferait disparaître le
symptôme en gardant la confusion : une quantité d'achat n'a aucune raison de
suivre le plafond d'un roster.

## Vérifié au passage

Les **coups de pouce ordinaires ne sont pas touchés** aujourd'hui : le corpus
d'exemple plafonne à `maxQuantity = 5`. Mais `build_allowed_specs`
(`record_inducements_use_case.rs:316` et `:324`) porte le **même motif** —
`InducementQty::try_new(s.max_qty).ok()?` dans un `filter_map`. Un corpus qui
déclarerait un inducement à plus de dix exemplaires le ferait disparaître de la
même façon, sans un mot. À corriger avec le reste, tant qu'on y est.

## Checklist

- [x] Le plafond cesse d'être un `InducementQty` — c'est un `InducementMaxQty`,
      sans borne haute
- [x] Un achat ne peut plus exister sans sa spec — construction conjointe
- [x] `build_purchase_list` refuse (`UnknownInducement`) au lieu de filtrer ; le
      contrôleur en fait un 422 avec une ligne `warn`
- [x] Même traitement pour `build_allowed_specs`
- [x] Cinq tests unitaires, dont celui qui reproduit le défaut — **vu échouer**
- [x] Test e2e, **vu échouer** : `"purchases": []`
- [x] `make lint`, `make check-arch`, `make test` — 1271 tests

## Ce qui a été fait

Le plafond a son type, `InducementMaxQty`, **sans borne haute** : il vient du
corpus, pas d'une saisie. `InducementQty` garde la sienne à dix, qui est un
plafond d'achat et n'avait aucune raison de suivre celui d'un roster. Le
compilateur a désigné les trois sites à basculer.

Spec et achat se construisent ensemble, dans un `let … else` qui journalise et
passe au suivant. L'orphelin ne peut plus naître.

### Il y avait un troisième avalement

La carte en décrivait deux ; `validate_max_qty` porte le même `if let` sans
`else`. Un achat sans spec y passait la validation en silence avant même
d'atteindre le `filter_map`. Il est désormais refusé **au plus tôt** — c'est la
phase de validation, pas celle de construction — et `build_purchase_list`
refuse à son tour, ce qui ferme la chaîne des deux côtés.

### Le contrôleur ne rendait pas 422

Un refus du domaine tombait dans le `Err(e)` générique, donc en **500 muet**.
Il rend maintenant un 422 avec le message du domaine et une ligne `warn`
nommant le rapport, l'équipe et le motif.

### Le corpus de démonstration ne peut pas exprimer le cas

Vérifié : **seuls les postes journalier dépassent dix** dans le jeu d'exemple —
et un test existant vérifie que la grille des mercenaires les écarte. Le défaut
n'est donc pas reproductible par l'interface avec ce corpus.

Mais le **serveur ne les filtre pas** : `validate_mercenary_positions` vérifie
seulement que le poste appartient au roster. La production le prouve. Le test
e2e poste donc en HTTP, ce qui reproduit le cas réel sans déplacer les plafonds
du corpus — dont plusieurs tests de valeur d'équipe dépendent.

Sans le correctif, il rend :

```
'{"type": "InducementsRecorded", "purchases": [], …}'
```

L'événement émis, vide, et le mercenaire facturé évaporé.
