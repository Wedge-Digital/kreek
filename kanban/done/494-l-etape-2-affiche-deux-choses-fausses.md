# L'étape 2 du rapport affiche deux choses fausses

**Priorité : haute** — l'une est invisible en local et systématique en jeu réel
**Contexte :** `match_report` · **Sans épic** · **Demandée par :** l'utilisateur

## Ce qui n'est pas en cause

Le **mécanisme** de TV est correct, vérification faite. L'écran refait ses deux
`fetch` à chaque chargement, donc il lit la valeur vivante ; le rapport
enregistre ses TV au moment de la soumission du fan factor, en relisant la
valeur vivante à cet instant (`record_fan_factor_use_case.rs:42`) ; et ce sont
celles-là que `topdog_team_id()` utilise pour les coups de pouce. Vraie TV à la
saisie, vraie différence pour le budget.

Une première analyse proposait d'afficher l'instantané enregistré plutôt que la
valeur vivante. **Écartée** : elle aurait figé l'écran sur une valeur périmée,
exactement l'inverse du besoin.

## Défaut 1 — `formatKpo` mange le chiffre des dizaines

`pre-match.html:27`. La fonction voulait insérer une espace de milliers. Au lieu
de séparer les chiffres, elle **recompose le nombre** depuis les milliers et les
centaines :

```js
const maj = Math.floor(v / 1000);          // 2075 → 2
const min = Math.floor((v % 1000) / 100);  // 2075 → 0
return min > 0 ? maj + ' ' + min + '00 kPo' : maj + ' 000 kPo';  // → "2 000 kPo"
```

Le chiffre des dizaines n'apparaît dans **aucune** des deux branches. Il est
perdu par construction.

| réel | affiché | |
|---|---|---|
| 990 | 990 kPo | ✓ |
| 1 050 | 1 000 kPo | ✗ |
| 1 250 | 1 200 kPo | ✗ |
| 2 075 | 2 000 kPo | ✗ |

Toute valeur ≥ 1 000 non multiple de 100 est fausse. **Cinq appels** sont
touchés : les deux TV d'équipe, la différence affichée, et les deux budgets de
trésorerie.

**Pourquoi rien ne l'a vu.** La TV maximale de la base locale est **990** : les
équipes des fixtures e2e ne franchissent jamais le seuil où le bug commence. Il
est invisible en local et systématique dès qu'une équipe a joué.

**Elle n'a jamais eu de raison d'être.** `formatKpo` n'existe que dans ce
fichier, introduite par le commit qui crée la page (`e4f8c63`, 25 juin). Les
vingt-six autres affichages de kPo de l'application impriment la valeur brute.
Cet écran était le seul à formater, et le seul à se tromper.

Ce qu'il faut garder d'elle : le `'…'` quand la valeur est nulle. Les cinq
appels s'exécutent avant que les `fetch` aient répondu, et afficheraient
« null kPo » sans lui.

## Défaut 2 — le jet de fan factor est toujours à 2

Deux valeurs en dur, à deux endroits :

```html
x-data="{ homeRoll: 2, awayRoll: 2, … }"
<input name="home_fan_roll" … value="2" x-model="homeRoll" required>
```

Et le contrôleur **jette les valeurs enregistrées**. Le domaine porte
`home_fan_roll: Option<D3Roll>` et `away_fan_roll: Option<D3Roll>` ;
`pre_match_controller.rs:87` ne s'en sert que pour un booléen :

```rust
pm.home_fan_roll.is_some() && pm.away_fan_roll.is_some()
```

D'où deux symptômes d'un seul défaut : rapport vierge → affiche 2 au lieu de
rien ; rapport déjà saisi → affiche 2 au lieu de ce qui a été enregistré. Le
bandeau « déjà enregistré » s'affiche correctement par-dessus, ce qui rend la
contradiction visible à l'écran.

`required` reste : un champ vide bloque la soumission, ce qui est le sens de
« vide tant que rien n'est saisi ».

*Vérifié et écarté* : le contrôleur passe `DedicatedFans::default()` dans la
commande, ce qui a l'air d'un bug — mais le use case les écrase avec les vraies
valeurs lues au port (`record_fan_factor_use_case.rs:42-44`). Déroutant, pas
faux.

## Ce que la carte ne fait pas

**`player_count` compte les morts.** `team_match_context_widget.rs:44` prend
`find_squad(...).len()`, tous membres confondus, là où le domaine a
`Squad::size()` qui les exclut depuis la carte 488. Le bloc « Journaliers » du
même écran propose donc un journalier de moins pour une équipe endeuillée —
l'inverse de la décision prise en 488. **Défaut réel, hors périmètre**, à sa
propre carte.

**Les dix-neuf équipes à `team_value = 0`** de la base locale — effectif complet,
TV jamais calculée. Portée non établie : mesurée en local uniquement, ni la démo
ni la production n'ont été interrogées.

**L'affichage d'une TV nulle.** `if (!v && v !== 0)` rend « 0 kPo » là où « … »
serait plus honnête. Choix d'affichage, pas un bug.

## Tests

Le test de `formatKpo` doit porter sur une valeur **à dizaines non nulles** —
2075. Avec 2000 ou 1200 il passerait sur le bug.

## Checklist

- [x] `formatKpo` imprime la valeur brute, garde le `'…'` sur nul
- [x] Le contrôleur expose les deux jets en `Option<u8>` au lieu du seul booléen.
      **Trouvé en chemin** : `ReadyToPublish` et `Published` rendaient `true` en
      dur pour le booléen, alors que les trois états portent les jets — vrai de
      l'enregistrement, muet sur ce qui a été enregistré
- [x] Le gabarit rend les jets enregistrés, et rien quand il n'y en a pas ; les
      totaux se gardent du `NaN` que `parseInt('')` produit sur un champ vide
- [x] Les deux `value="2"` disparaissent, ainsi que les défauts de l'`x-data`
- [x] Trois tests unitaires de gabarit — vierge, saisi, plus d'arithmétique de
      recomposition. **Falsifiés** : les trois échouent sur le gabarit d'origine
- [x] Deux tests e2e. Le premier enregistre ses propres jets avant de rouvrir la
      page, donc indépendant de l'ordre. Le second éprouve `formatKpo` **dans le
      navigateur** via `Alpine.$data`, sur 2075, 1250, 1000, 990, 0 et `null` —
      lire la TV à l'écran ne prouverait rien, aucune équipe locale n'atteignant
      le seuil de 1 000 où le défaut commence
- [x] `make lint`, `make check-arch` (17 axes), `make test` (1635),
      `make e2e` (199 passés sur la sélection impactée, 0 échec)

## Terminé quand

Sur la base de démonstration : rouvrir l'étape 2 d'un rapport où le fan factor a
été saisi à 1 et 3 réaffiche 1 et 3, et un rapport neuf présente deux champs
vides. Une équipe à 2 075 kPo s'affiche « 2075 kPo ».
