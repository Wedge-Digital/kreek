# L'écran du jet · Phase 8 : cartes kanban

**Entrée** : phases 2 à 7 validées. Dernière phase de conception.

## Quatre cartes, strictement séquentielles

| # | Carte | Livrable observable |
|---|---|---|
| **408** | Le domaine des erreurs coûteuses | la table, les effets, la phase, les deux sorties de la validation — **36 tests** |
| **409** | Lancer le dé par requête | port du dé, use case, POST gardé — sans écran |
| **410** | L'écran du jet | page, fragment, animation, CSS, bandeau |
| **411** | Les erreurs coûteuses sous Playwright | six scénarios |

```
408 ──► 409 ──► 410 ──► 411
```

Aucun parallélisme possible : chaque carte a besoin de la précédente. La Haine
pouvait fourcher — l'écran et la traversée vers `players` ne se touchaient pas —
ici tout est dans `teams` et tout s'empile.

## Deux écarts assumés

**La 408 embarque l'adaptation du use case de validation des renvois.** Il rend
`Result<(), …>` et devra rendre l'issue : sans propager, le projet ne compile
plus. Le prix d'une signature qui change se paie dans la carte qui la change.

**La 409 livre quelque chose d'invisible** — un jet qu'aucun écran ne déclenche,
testable par requête seulement. La fusionner avec la 410 donnerait un livrable
qu'on peut voir marcher, mais une carte portant le port, le use case, le
handler, la page, le fragment, le CSS et le bandeau. Elles restent séparées.

## La dépendance qui n'est pas une carte

La **409 a besoin de `ITeamAccessPort`**, celui de la carte **389** — coach
propriétaire, admin d'espace, admin de compétition. Si la 389 est livrée avant,
le port existe ; sinon la 409 le crée et la 389 s'en sert. C'est écrit dans les
deux cartes.

## Ce qui reste hors du lot

- **L'onglet Trésorerie** (carte 48) : la ligne du mouvement existe déjà au grand
  livre, il manque l'écran qui la montre. C'est ce qui rendra le jet consultable
  après coup — la fonctionnalité ne le fait pas.
- **La retraite temporaire** (carte 39), toujours hors du chemin des phases.
- **La consultation du jet passé**, écartée en phase 2.

## Ce que le lot ne peut pas garantir seul

Le dé est **réellement aléatoire** : les tests e2e de la 411 ne peuvent rien
affirmer sur l'issue sans devenir instables une fois sur six. C'est la 408 et ses
36 cas qui prouvent la table ; la 411 ne vérifie que la cohérence entre ce que
l'écran annonce et ce que la trésorerie devient.

Cette répartition est délibérée, et elle explique pourquoi les 36 cas ne sont pas
négociables : **rien d'autre ne les couvre**.
