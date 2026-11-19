# L'écran du jet

**Priorité : haute**
**Dépend de :** 409
**Conception :** `docs/specs/erreurs-couteuses/ecran-du-jet/{02-front,07-integration}.md`
**Maquette :** `assets/rawpages/html/app-team-costly-mistakes.html`
**Fichiers :** `src/app/teams/io/web/costly_mistakes.rs`,
`io/web/templates/teams-costly-mistakes.html` (nouveau),
`io/web/templates/teams-costly-mistakes-result.html` (nouveau),
`io/web/team_detail.rs`, `routes.rs`,
`assets/static/css/pages/costly-mistakes.css` (nouveau), `src/web/css_bundle.rs`

## Objectif

Le coach voit sa tranche, lance le dé, et lit ce qu'il lui en coûte.

## Aucun widget

Le `CLAUDE.md` réserve le pattern d'assemblage aux pages de trois sections
interactives et plus, et nomme le cas contraire : « page simple avec un
formulaire et une réponse ». C'est celle-ci — une table statique, un bouton, une
réponse, tout venant de `teams`.

| Élément | Chargement |
|---|---|
| En-tête, table, zone de jet | avec la page |
| Résultat | **fragment**, en réponse au POST de la carte 409 |

## L'animation a besoin d'une durée plancher

Le POST répondra en quelques dizaines de millisecondes. Sans plancher — un peu
plus d'une seconde — **le dé clignoterait et le résultat surgirait** sans que
rien ne se soit passé.

Ce n'est pas de la décoration : le coach ne lance pas le dé lui-même, et le
suspense est la seule chose qu'il obtienne en échange. Le fragment est donc reçu
puis **affiché à l'échéance** — `htmx.ajax` piloté par le composant Alpine qui
tient l'animation, et non un `hx-post` nu qui échangerait aussitôt.

## La table est affichée, et ce n'est pas décoratif

Le coach ne tire pas lui-même : à défaut d'avoir prise sur le résultat, il doit
pouvoir **vérifier qu'il est juste**. Sa tranche est mise en évidence avant le
jet ; la case atteinte l'est après.

Le fragment renvoie donc **aussi la table**, avec sa case touchée : réafficher
six lignes coûte moins qu'un second échange pour mettre à jour un tableau.

## Le calcul est montré en entier

```
Trésorerie                          345 kPo
La moitié                         172,5 kPo
Arrondie au 5 kPo inférieur         170 kPo
─────────────────────────────────────────
Perte                             − 170 kPo
Il vous reste                       175 kPo
```

Les dés secondaires s'affichent en petit à côté de leur ligne. Un coach qui perd
340 kPo doit pouvoir refaire l'opération sans demander à personne.

**Le calcul est une liste de lignes, pas quatre champs nommés** : chaque incident
a son enchaînement, et un VM à champs fixes obligerait le template à savoir
lequel afficher.

## La page n'existe que pendant la phase

Hors `CostlyMistakes` → **422**, comme le fait déjà `dismissals.rs:70`. Cette
famille d'écrans n'a pas de sens hors de sa phase.

**Conséquence assumée** : un coach qui recharge après le jet ne reverra pas son
résultat. Le montant figure au grand livre avec le motif `CostlyMistake`, et sera
lisible quand l'onglet Trésorerie existera (carte 48).

## Le bandeau y mène

```rust
(Enrolled, Some(CostlyMistakes)) => Some(BannerVm {
    icon: "💸", title: "Erreurs coûteuses.",
    ctas: vec![BannerCtaVm::Navigate { label: "Lancer le dé →", href: … }],
})
```

Une branche de plus dans `BannerVm::from_domain`, avec le type de CTA **qui
existe déjà** et sert au recrutement comme aux renvois.

## Conventions

- Le fragment **ne répète pas l'`id` de son conteneur** — piège documenté du
  `CLAUDE.md` pour les injections `innerHTML`.
- L'animation vit dans un `x-data`, pas dans un `<script>` nu ; aucun `id`
  global (conventions 6 et 7).
- La feuille CSS est **scopée sous `.cm-page`** et **inscrite au bundle** :
  l'axe 14 de `check-arch` refuse toute feuille absente de la liste.

## Checklist

- [ ] Route `COSTLY_MISTAKES_PAGE`, handler avec garde de phase (422)
- [ ] Template de page : en-tête, table, zone de jet, bouton de sortie désactivé
- [ ] Template de fragment : verdict, calcul, table avec la case touchée
- [ ] `BandVm`, `CalcLineVm` — le calcul en liste
- [ ] Animation avec durée plancher, en Alpine
- [ ] Branche de bandeau dans `team_detail.rs`
- [ ] Feuille CSS scopée **et inscrite au bundle**
- [ ] Tests unitaires : les six `BandVm` avec la bonne tranche courante ; les
      lignes de calcul pour les quatre incidents
- [ ] Vérifié à l'écran sur le serveur de développement, les quatre issues
- [ ] `make lint`, `make check-arch`, `make test`
