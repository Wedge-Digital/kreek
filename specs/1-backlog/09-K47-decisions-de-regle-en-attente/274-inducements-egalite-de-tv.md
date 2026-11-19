# Coups de pouce à égalité de valeur d'équipe — décider la règle

**Priorité : basse** — cas rare, mais la règle actuelle n'a jamais été décidée
**Découverte :** en corrigeant l'influence des coups de pouce sur la trésorerie
**Fichiers pressentis :** `src/app/match_report/domain/match_report_pre_match.rs`

## Problème

À valeurs d'équipe **égales**, le code désigne l'équipe **domicile** comme top
dog :

```rust
pub fn topdog_team_id(&self) -> &TeamId {
    if away_tv > home_tv { &self.away_team_id } else { &self.home_team_id }
}
```

Ce n'est pas une décision : c'est la conséquence d'un `>` strict. Personne n'a
tranché ce que le jeu doit faire dans ce cas.

Conséquences telles quelles :

- l'équipe **extérieure** devient underdog et reçoit une petite monnaie de
  `0 + dépenses du top dog` — donc de l'argent gratuit dès que l'équipe
  domicile achète quoi que ce soit ;
- elle peut en outre compléter avec 50 kPo de sa trésorerie, quand l'équipe
  domicile n'a droit qu'à sa caisse ;
- l'avantage bascule intégralement sur un critère arbitraire — qui reçoit
  l'étiquette « domicile » dans le rapport de match.

Deux équipes de valeur identique ne sont, par définition, ni favorite ni
outsider. La règle actuelle en désigne pourtant une.

## Ce qu'il faut décider

Trois lectures possibles, à trancher avec les règles du jeu en main :

1. **Ni top dog ni underdog** — chacun n'achète qu'avec sa trésorerie, aucune
   petite monnaie. C'est l'intuition la plus directe, et elle demande un
   troisième cas dans `inducement_budget_for` et `treasury_spending_for`.
2. **Le statu quo, assumé** — le domicile est top dog, et on l'écrit
   explicitement dans le code et les règles au lieu de le subir.
3. **Un départage** par un autre critère — classement, tirage, choix des coachs.
   Demande de savoir si le règlement en prévoit un.

## Ce que la correction de trésorerie a déjà mis en place

Le calcul de ce qui sort réellement de la caisse est isolé dans
`treasury_spending_for`, et la petite monnaie dans `petty_cash`. Le cas d'égalité
se traitera donc à deux endroits nommés, sans toucher au reste.

Le test e2e `test_inducement_treasury` **échoue explicitement** si les deux
équipes de la fixture ont la même valeur, avec un message qui renvoie à cette
carte : le scénario n'a plus d'objet sans écart, et il vaut mieux qu'il le dise
que de vérifier une règle non décidée.

## Questions ouvertes

- Que dit le règlement Blood Bowl pour deux équipes de TV strictement égale ?
- Le cas est-il atteignable en pratique dans une ligue, ou seulement en début
  de saison quand toutes les équipes sortent du même budget de création ?
  *(Piste : les équipes de démo `DEMO_GRANIT` et `DEMO_ZEPHYR` n'ont pas la même
  valeur, mais deux équipes du même roster construites à l'identique, si.)*
