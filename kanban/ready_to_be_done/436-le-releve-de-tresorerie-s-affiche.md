# Le relevé de trésorerie s'affiche

**Épic :** E06 — La fiche d'équipe complétée · **Ordre :** 2 · **Dépend de :** 434, 435
**Conception :** `docs/specs/tresorerie-equipe/onglet-tresorerie/` (`04-dtos.md`,
`07-integration.md`) · **Maquette :**
`assets/rawpages/html/app-team-treasury.html`

## Objectif

Rendre le relevé à l'écran, et câbler l'onglet qui y mène. C'est la carte qui
livre la fonctionnalité.

## Conception

### Le handler

```rust
// teams/io/web/treasury_tab.rs
pub async fn treasury_tab(
    auth_session: AuthSession,
    Path((space_id, team_id)): Path<(String, String)>,
    State(state): State<AppState>,
) -> Response;
```

**Aucune route nouvelle** : la 434 a posé `TEAM_TREASURY`, et le handler
distingue les deux chemins à l'en-tête `HX-Request` — fragment nu pour le clic
sur l'onglet, page entière pour un lien collé. **Un seul gabarit de fragment
pour les deux**, comme le fait l'administration de compétition.

**Aucun contrôle d'accès nouveau** : celui de la fiche équipe s'applique tel
quel. La trésorerie n'est pas plus sensible que la valeur d'équipe, déjà
affichée dans l'en-tête à qui voit la page.

| Cas | Réponse |
|---|---|
| Nominal | le fragment |
| Aucun mouvement au-delà de la dotation | le même fragment, bloc « Aucun mouvement pour l'instant » |
| `MissingOpeningEntry` | `500` + journal `ERROR` |
| `UnknownReason(motif)` | `500` + journal `ERROR` **portant le motif** |

**`500` et non `422`.** Un `422` dirait au coach qu'il a mal fait quelque chose ;
il n'y est pour rien. Ces deux cas décrivent une base qui ne devrait pas
exister, et la seule action utile est qu'ils apparaissent au journal avec leur
`rid` — le motif compris, puisque c'est lui qu'on cherchera.

### Les view models

```rust
pub struct TreasuryVm {
    pub summary: SummaryVm,
    pub groups: Vec<GroupVm>,
    pub is_opening_only: bool,
    pub movement_count: u32,
}
pub struct SummaryVm { pub opening_kpo: u32, pub credited_kpo: u32,
                       pub debited_kpo: u32, pub balance_kpo: u32 }
pub struct GroupVm  { pub heading: Option<String>, pub rows: Vec<MovementRowVm> }
pub struct MovementRowVm {
    pub date_label: String, pub icon: &'static str,
    pub label: String, pub detail: Option<String>,
    pub amount_label: String, pub balance_label: String,
    pub kind: RowKind,
}
pub enum RowKind { Opening, Credit, Debit, Correction }
```

**`RowKind` et non un `css_class: String`.** Un view model qui porte des noms de
classes fige la présentation dans le code Rust — changer une couleur demanderait
de recompiler. L'énumération dit *ce que la ligne est* ; le gabarit choisit
`tr-row--fix` ou `tr-icon--credit`.

**`amount_label` porte son signe, `balance_label` non** : le signe appartient au
montant, un solde est un état.

**`detail: Option<String>` et non `String`** : trois motifs n'ont pas de détail,
et une chaîne vide laisserait un `<div>` qui prend sa marge.

**`heading: Option<String>`** : le groupe d'ouverture n'a pas de journée, et le
gabarit n'affiche alors aucun séparateur.

### Construction

`TreasuryVm` dépend d'un port autant que du domaine : il **ne peut pas** exposer
un `from_domain()`. Sa construction vit dans `builders.rs` (`CLAUDE.md`).

```rust
pub fn build_treasury_vm(statement: &TreasuryStatement) -> TreasuryVm;
```

**Tout le formatage est ici, et nulle part ailleurs** — dates, montants, les
huit libellés de motif, les emojis, « Journée 1 — contre les Trolls du Bief ».
Ni le service, qui rend des types, ni le gabarit, qui n'a aucune logique. C'est
ce qui rendra la traduction possible le jour venu : un seul fichier à toucher
pour cet écran.

### Le gabarit

`teams/io/web/templates/teams-treasury-tab.html`, d'après la maquette.

Un seul conditionnel : `is_opening_only` choisit entre le tableau et le bloc
« Aucun mouvement pour l'instant ». Le bandeau de synthèse s'affiche dans les
deux cas.

**Aucun JS, aucun Alpine, aucun `hx-disinherit`.** Le dernier protégerait d'un
danger qui n'existe pas : la fiche équipe ne pose aucun `hx-vals` ni
`hx-include` sur son conteneur.

### L'onglet devient cliquable

Dans `teams-team-detail.html`, « Trésorerie » passe de `<div>` inerte à `<a>`
htmx, sur le modèle posé par la 434 pour « Joueurs & Staff ».

**« Matchs » reste inerte.**

### Le style

`assets/static/css/pages/team-treasury.css`, portée par `.treasury`, **inscrite
dans `src/web/css_bundle.rs` juste après `pages/team-page.css`** (ligne 112) :
l'ordre du bundle est imposé et les deux feuilles servent la même page. L'axe 14
de `check-arch` refuse une feuille absente du bundle.

Sous 768 px, **la colonne « Solde après » disparaît avant les montants** : le
solde courant reste lisible dans le bandeau, le montant d'un mouvement n'a aucun
autre endroit où se lire.

## Ce que la carte ne fait pas

- **Aucune écriture**, aucun événement, aucune migration.
- **Aucune pagination** : quelques dizaines de lignes par saison.
- **Aucun tri, aucun filtre.**

## Tests

- **Unitaires de `builders.rs`** : les huit libellés de motif, le signe des
  montants, le repli de `detail` sur `None`, le groupe d'ouverture sans
  `heading`, `is_opening_only` sur un relevé à une ligne.
- **Pas de test unitaire de handler** : il n'orchestre rien que le service ne
  fasse, et la 435 le couvre.

Les tests de navigateur sont la carte 437.

## Checklist

- [ ] `treasury_tab.rs` et sa distinction `HX-Request`
- [ ] Les view models et `build_treasury_vm`
- [ ] `teams-treasury-tab.html` d'après la maquette
- [ ] « Trésorerie » en `<a>` htmx
- [ ] `pages/team-treasury.css` + son inscription au bundle
- [ ] `make lint && make test && make check-arch`
