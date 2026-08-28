# Le classement affiche les points manuels

**Ordre :** 2 · **Dépend de :** 449, 450 · **Prérequis : la carte 448**
**Conception :** `docs/specs/points-classement-manuels/page-de-gestion/`
(`04-dtos.md`, `07-integration.md`) · **Maquette :**
`assets/rawpages/html/app-competition-detail.html`

## Objectif

Rendre les points manuels visibles dans les deux vues du classement, et les
faire entrer dans l'ordre.

## La carte 448 passe avant, ou avec

`widgets/ranking-detailed-standings-widget.css` met le zébrage en `--dark-7`
(ligne 40) et le survol en `--dark-6` (ligne 44) — **deux valeurs séparées par
un rapport de contraste de 1,0012**, c'est-à-dire indiscernables.

**Le survol du classement détaillé est invisible une ligne sur deux, en
production, aujourd'hui.**

Ajouter une colonne à ce tableau sans corriger cela livrerait une nouveauté dans
un écran déjà cassé. Ce n'est pas un couplage de convenance : c'est le même
fichier, et la 451 n'a pas de sens sans elle.

## Conception

### Une lecture de plus, en parallèle

`classement_widget.rs:53` et `detailed_standings_widget.rs:96` ajoutent
`find_manual_totals_for_season` à leur `tokio::join!` existant :

```rust
let (rules, teams, lines, groups, manual) = tokio::join!(…);
…
build_ordered_standings(lines, &manual, &order)
```

**Quatre requêtes deviennent cinq, en parallèle** — le temps de réponse ne
bouge pas.

### Les deux view models

```rust
// classement_widget.rs
pub struct ClassementRowVm {
    …,
    pub points: u32,               // inchangé : les points de match
    pub manual: Option<i32>,       // ← None si l'équipe n'en a aucun
    pub total: i32,                // ← était u32
}

// detailed_standings_widget.rs
pub struct DetailedRowVm {
    …,
    pub bonus: String,             // inchangé, signé
    pub manual: Option<String>,    // ← « −1 », signé
    pub total: i32,                // ← était u32
}
```

**`Option` et non un zéro par convention.** Le gabarit doit distinguer « aucun
point manuel » — un tiret, non cliquable — de « zéro point manuel », **qui
n'existe pas** puisque `ManualPoints` le refuse. L'`Option` rend cette
impossibilité dans le type.

**`total` passe de `u32` à `i32`** : c'est le seul changement de type que la
fonctionnalité impose au code existant, et il découle d'un total qui peut
devenir négatif.

### Les deux colonnes

| Gabarit | Position | En-tête |
|---|---|---|
| `classement-widget.html` | entre `D` et `Pts` | « Man. » |
| `detailed-standings-widget.html` | entre `Bonus` et `Total` | « Manuel » |

**Dans le groupe « Points », jamais à côté des départages.** C'est la règle
rendue par la structure du tableau : les points manuels entrent dans le total
*avant* qu'on départage. Le `colspan` du groupe « Points » du détaillé passe de
2 à 3.

**Un point manuel non nul est un lien** vers la page de gestion — on voit un
chiffre qui surprend, on veut savoir d'où il vient. Le tiret d'un zéro n'en est
pas un.

### Le bouton d'accès

Dans les **deux onglets de classement**, et nulle part ailleurs. Il ne s'affiche
qu'aux administrateurs ; la page qu'il ouvre est consultable par tous.

**La duplication est délibérée, et le commentaire du gabarit doit le dire** : la
page de compétition compte six onglets, et au-dessus d'eux le bouton
s'afficherait sur Calendrier, Équipes et Statistiques, où il n'a rien à faire.
Sans ce commentaire, quelqu'un la « factorisera » dans six mois.

### Responsivité

Sous 768 px, **la colonne « Man. » reste**. C'est une composante du total, et la
masquer rendrait le total inexplicable — exactement ce que la fonctionnalité
cherche à réparer.

## Tests

Unitaires, sur les builders :

| Test | Ce qu'il prouve |
|---|---|
| `une_equipe_sans_point_manuel_rend_none` | le tiret, pas un zéro |
| `le_total_affiche_inclut_le_point_manuel` | l'addition arrive au VM |
| `un_total_negatif_se_rend_signe` | « −2 » |

Les tests de navigateur sont la carte 453.

## Checklist

- [ ] **La carte 448 est passée**
- [ ] Les deux `tokio::join!` et l'appel à `build_ordered_standings`
- [ ] Les deux VM, `manual: Option`, `total: i32`
- [ ] Les deux colonnes, `colspan` du détaillé corrigé
- [ ] Le lien sur un point non nul
- [ ] Le bouton dans les deux onglets, avec son commentaire
- [ ] `make lint && make test && make check-arch`
