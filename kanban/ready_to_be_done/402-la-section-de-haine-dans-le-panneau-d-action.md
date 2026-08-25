# La section de Haine dans le panneau d'action

**Priorité : haute**
**Dépend de :** 401
**Conception :** `docs/specs/haine/saisie-des-actions/02-front.md`
**Maquette :** `assets/rawpages/html/app-match-report-step3-haine.html`
**Fichiers :** `src/app/match_report/ports.rs`,
`src/infrastructure/match_report/ref_team_data_adapter.rs`,
`use_cases/hate_keywords_service.rs`,
`io/web/widgets/action_panel_widget.rs`,
`io/web/templates/action-panel-widget.html`,
`io/web/templates/action-log-widget.html`,
`assets/static/css/…`

## Objectif

Le coach déclare la Haine à l'écran, et la voit au journal des actions.

## Aucun widget nouveau

La section vit **dans `action-panel-widget`**, qui porte déjà `showInjury`,
`showSequel`, `injuryType` et `sequelStat`. Trois raisons, établies en phase 2 :
le panneau est déjà rechargé à chaque changement de tour ou de joueur, un widget
dédié devrait recevoir son contexte d'un autre widget — ce que la règle 2
interdit —, et l'union des mots-clefs adverses ne bouge pas de tout le match.

Le handler cesse d'être inerte : il commence aujourd'hui par `let _ = state;`.

## Le domain service

```rust
// use_cases/hate_keywords_service.rs
pub struct HateKeywordChoices {
    pub in_opponent_roster: Vec<Keyword>,
    pub others: Vec<Keyword>,
}
```

Il partage le catalogue sur l'union des `keywords` du **roster** adverse —
`RosterPositionDto` gagne le champ, l'adapter le remplit — et **trie par
libellé** : un coach cherche « Nain », pas « DWARF ».

**Le service est obligatoire** : sans lui le handler manipulerait les DTOs du
port pour en faire des VMs, ce que le `CLAUDE.md` interdit nommément.

Le partage se fait sur le roster entier, pas sur les joueurs alignés : couvrant,
et sans dépendance à la feuille de match. D'où le titre du groupe — **« Dans le
roster adverse »**, et non « rencontrés », qu'un poste non aligné démentirait.

## Les quatre comportements à reprendre de la maquette

1. La section n'apparaît que sur **Amoché, Blessure Sérieuse, Séquelle** —
   constante nommée, jamais trois comparaisons dispersées.
2. Le bouton de confirmation **reste masqué** tant que la question n'est pas
   tranchée, et tant qu'aucun mot-clef n'est choisi si la réponse est oui.
3. Le filtre **ouvre le repli de lui-même** quand le mot cherché ne s'y trouve
   que là. Sinon le coach tape « yéti », voit une liste vide, et conclut que le
   mot n'existe pas.
4. Le mot-clef choisi **reste visible** même s'il ne correspond plus au filtre,
   et le groupe du roster adverse disparaît quand il devient vide plutôt que de
   laisser un titre au-dessus du rien.

## Les deux listes voyagent en HTML

Pas de JSON, pas d'endpoint : le filtre travaille sur le DOM déjà rendu. C'est la
conséquence du choix de ne pas créer de widget — et ce qui évite toute latence au
moment du clic.

## Conventions à tenir

`hx-disinherit="*"` déjà présent, à conserver. Les quatre propriétés nouvelles
rejoignent le `x-data` existant — pas de `<script>` nu, pas d'`id` global
(conventions 6 et 7). Le CSS rejoint une feuille **inscrite au bundle**
(`css_bundle.rs`), sous une portée nommée d'après sa racine — l'axe 14 de
`check-arch` refuse toute feuille absente de la liste.

## Le journal des actions

La ligne de blessure affiche « + Haine : Nain ». C'est cinq lignes de template
dans un autre widget : elles sont ici plutôt que dans une carte à elles, où la
cérémonie coûterait plus que le contenu.

## Checklist

- [ ] `RosterPositionDto.keywords`, rempli par `ref_team_data_adapter`
- [ ] `hate_keywords_service` + tests de partage et de tri
- [ ] `action_panel_widget` appelle le service, porte les deux listes en VMs
- [ ] Template : question, filtre, deux groupes, repli, confirmation conditionnelle
- [ ] Journal des actions : la Haine sur la ligne de blessure
- [ ] Feuille CSS scopée **et inscrite au bundle**
- [ ] Tests unitaires : partage roster adverse / autres, tri par libellé,
      adversaire sans mot-clef connu → premier groupe vide
- [ ] `make lint`, `make check-arch`, `make test`
