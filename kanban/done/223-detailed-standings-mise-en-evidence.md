# Classement détaillé — Mise en évidence du critère décisif

**Priorité : moyenne — reportable**
**Dépend de :** cartes 220 (`tiebreak_outcomes`) **et** 222 (onglet livré)
**Contexte :** `src/app/ranking/io/web/builders.rs`, `src/app/ranking/io/web/widgets/detailed_standings_widget.rs`, `src/app/ranking/io/web/templates/widgets/detailed-standings-widget.html`, `assets/static/css/widgets/detailed-standings-widget.css`
**Spec :** `docs/specs/ranking/tiebreakers/detailed-standings/06-domaine.md`

## Objectif

Colorer le critère qui a effectivement départagé chaque égalité de points, griser ceux qui
n'ont pas tranché, et rendre l'ex æquo total visible. Implémente les **règles 21 et 22**
côté présentation.

**Carte volontairement isolée** : si elle se révèle plus retorse que prévu, elle se
reporte sans rien bloquer. L'onglet livré par la 222 reste exploitable — il perd son
commentaire visuel, pas sa fonction.

## Pourquoi elle vaut la peine

Sans elle, l'onglet affiche des colonnes de chiffres et laisse le lecteur refaire la
comparaison de tête. C'est précisément ce que la maquette a été dessinée pour éviter : on
répond à « pourquoi cette équipe est-elle devant celle-là ? » en désignant le critère,
pas en fournissant les données brutes.

## Conception

### Câblage

`builders.rs` appelle `tiebreak_outcomes` (carte 220) sur les standings ordonnés de
**chaque poule** — une poule est un classement autonome, la résolution s'y fait
indépendamment — puis traduit chaque `RowTiebreak` en états de cellules :

| `RowTiebreak` | Colonnes avant l'index | Colonne à l'index | Colonnes après |
|---|---|---|---|
| `DecidedBy(k)` | `Tied` | `Decisive` | `Neutral` |
| `FullyTied` | `Tied` | — | `Tied` |
| `Alone` | `Neutral` | — | `Neutral` |

Le champ `CellState` existe depuis la 222 avec `Neutral` partout : cette carte le peuple.

### Rendu

`CellState::css_class()` fait la correspondance vers `sd-decisive` / `sd-tied` /
`""` — en un seul endroit. Ne pas disséminer de noms de classes CSS dans le builder : un
renommage CSS resterait alors invisible au compilateur.

Les deux classes s'ajoutent au CSS de la widget, reprises de la maquette.

### Légende

Complétée avec les deux phrases que la 222 avait laissées de côté :

- le critère mis en évidence est celui qui a départagé l'égalité de points ;
- deux équipes restent ex æquo lorsque tous les critères activés donnent la même valeur.

## Tests

- `DecidedBy(k)` ⇒ colonnes avant en `Tied`, colonne `k` en `Decisive`, après en `Neutral`
- `FullyTied` ⇒ toutes les colonnes en `Tied`, aucune `Decisive`
- `Alone` ⇒ toutes en `Neutral`
- La résolution est faite **par poule** : deux poules aux totaux identiques ne
  s'influencent pas

## Checklist

- [ ] `tiebreak_outcomes` appelé par poule, pas globalement
- [ ] Traduction `RowTiebreak` → `CellState` conforme au tableau ci-dessus
- [ ] `css_class()` seul point de correspondance vers les noms de classes
- [ ] Classes `sd-decisive` / `sd-tied` dans le CSS de la widget
- [ ] Légende complétée
- [ ] Tests ci-dessus
- [ ] `make test` + `make check-arch` passent
