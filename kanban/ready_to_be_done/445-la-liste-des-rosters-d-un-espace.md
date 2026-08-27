# La liste des rosters d'un espace

**Épic :** E10 · **Ordre :** 3 · **Dépend de :** 441, 442
**Conception :** `docs/specs/roster-personnalise/editeur-de-roster/`
(`02-front.md`, `07-integration.md`) · **Maquette :**
`assets/rawpages/html/app-roster-list.html`

## Objectif

Voir ce que les coachs d'un espace peuvent choisir, et ouvrir la porte de
l'éditeur. **Et poser le contrôle d'accès dont l'éditeur héritera.**

## Pourquoi cette carte vient avant l'éditeur

Elle est petite, il est gros. Mais **elle porte le résolveur `ISpaceOwnership`
pour `roster_uid`**, qui rend le contrôle d'accès structurel — un roster d'un
autre espace rend `404` **avant** le handler. L'éditeur en hérite gratuitement.

L'ordre inverse obligerait l'éditeur à contrôler à la main, puis à défaire.

## Conception

### Le résolveur — le geste qui compte

```rust
// infrastructure/references/space_ownership.rs
impl ISpaceOwnership for RosterSpaceOwnership {
    fn param(&self) -> &'static str { "roster_uid" }
    async fn space_of(&self, uid: &str) -> Option<SpaceId>;
}
```

Sur le modèle des six existants (`match_report_id`, `player_id`,
`competition_id`, `season_id`, `article_id`, `team_id`), enregistré dans
`main.rs`.

**Un uid du corpus n'appartient à aucun espace** : le résolveur rend `None`, que
le middleware traite comme `404`. C'est le comportement voulu — on ne consulte
pas un roster du règlement par cette route.

**Un doublon de paramètre est une erreur de démarrage**, pas un arbitrage
silencieux (`verifier_unicite_des_parametres`). `roster_uid` est libre.

### Les routes

```
GET /app/{space_id}/admin/rosters
```

Une seule pour cette carte. Le reste vient avec l'éditeur.

### Le handler

```rust
pub async fn roster_list(
    auth_session: AuthSession,
    Path(space_id): Path<String>,
    State(state): State<AppState>,
) -> Response;
```

**La lecture est ouverte à tout membre de l'espace** : la liste est ce qu'il
pourra choisir en créant une équipe. `can_manage` — admin d'espace — décide
seulement des **actions**.

### Le view model

```rust
pub struct RosterRowVm {
    pub uid: String, pub name: String, pub initials: String,
    pub tier: String, pub position_count: u32, pub reroll_cost: u32,
    /// `None` pour un roster du règlement.
    pub teams_using: Option<u32>,
    pub created_at: Option<String>,
}
```

**`teams_using: Option<u32>` porte la distinction des deux sections dans le
type.** Un `u32` avec zéro par convention laisserait le gabarit deviner entre
« aucune équipe » et « on ne compte pas ».

### Le gabarit

Deux sections, d'après la maquette : **les rosters de l'espace se gèrent, ceux
du règlement se consultent.** La première porte le poids visuel — filet
d'accent, ombre —, la seconde se tient en retrait.

**Le compteur d'équipes ne figure que sur les rosters de l'espace**, parce qu'il
n'y décide de quelque chose que là : zéro donne « Modifier » et « Supprimer »,
une ou plus ne laisse que « Consulter ».

**Les actions interdites n'existent pas, elles ne sont pas grisées** — un bouton
désactivé invite à chercher comment l'activer, alors qu'il n'y a rien à activer.
Le badge dit l'état, le compteur dit la cause.

### CSS

`pages/references-roster-list.css`, portée par `.rl-page`, **inscrite dans
`src/web/css_bundle.rs`** — l'axe 14 refuse une feuille absente du bundle.

## Tests

| Test | Ce qu'il prouve |
|---|---|
| `la_liste_montre_les_deux_sections` | unitaire, sur le VM |
| `un_roster_du_reglement_n_a_pas_de_compteur` | `teams_using` à `None` |
| `un_non_admin_ne_voit_pas_les_actions` | `can_manage` |
| `un_roster_d_un_autre_espace_rend_404` | **le résolveur**, avant le handler |

## Checklist

- [ ] Le résolveur `ISpaceOwnership` pour `roster_uid`, enregistré dans `main.rs`
- [ ] La route, le handler, le VM
- [ ] Le gabarit d'après la maquette
- [ ] La feuille + son inscription au bundle
- [ ] Les quatre tests
- [ ] `make lint && make test && make check-arch`
