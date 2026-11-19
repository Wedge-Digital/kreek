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

- [x] `treasury_tab.rs` — la distinction `HX-Request` existait déjà (carte 434)
- [x] Les view models et `build_treasury_vm`
- [x] `teams-treasury-tab.html` d'après la maquette
- [x] « Trésorerie » en `<a>` htmx
- [x] `pages/team-treasury.css` + son inscription au bundle
- [x] `make lint && make test && make check-arch` — et `make e2e`

---

# Ce que la réalisation a appris

## La maquette range la recette sous une journée que les données ne connaissent pas

La carte confie le regroupement au contexte de match. Seuls trois motifs en
portent un — coups de pouce, recette annulée, remboursement — et **la recette
d'après-match, la ligne la plus fréquente, n'en porte aucun** (trouvaille de la
carte 435 : l'information n'existe pas dans l'événement).

Un découpage naïf « le titre s'ouvre à la première ligne identifiable » place
donc chaque recette **au-dessus** du titre de son propre match, où elle se lit
comme appartenant au précédent. Ce n'est pas un cas limite : mesuré le
2026-08-30, le grand livre écrit la recette avant le paiement des coups de pouce
pour **110 équipes sur 110**, sans exception.

Le découpage livré remonte donc, avant d'ouvrir une période, la suite contiguë de
lignes qui appartiennent à une séquence d'après-match — recette et bourde
coûteuse, et elles seules. Un recrutement fait entre deux matchs n'est pas
absorbé par le suivant ; une ligne qui connaît déjà son match non plus.

Ce que ce découpage ne sait pas faire, et que rien dans les données ne permet :
deux matchs consécutifs dont le premier n'a donné lieu à aucun achat de coups de
pouce voient la recette du premier remonter dans la période du second.

**Ce défaut n'était visible qu'à l'écran.** Les tests unitaires écrits d'après la
carte passaient, parce qu'ils composaient les lignes dans l'ordre que la maquette
suggère et non dans celui que la base contient.

## Un défaut que seul l'écran pouvait montrer : l'onglet actif mentait

La carte 434 laisse le bandeau d'onglets hors de la cible du swap — « les onglets
restent dans `teams-team-detail.html`, qui n'est pas re-rendu au changement
d'onglet ». C'était **sans conséquence tant qu'un seul onglet menait quelque
part** : cliquer l'onglet déjà actif ne déplace rien.

Dès que « Trésorerie » devient cliquable, changer d'onglet échange le contenu
sans déplacer le soulignement : l'effectif s'affiche sous un onglet
« Trésorerie » actif. Rien ne le signalait — le HTML était valide, la route
correcte, l'URL poussée juste, et les six tests de la 434 verts.

Le bandeau entre donc dans la zone échangée (`teams-team-tab-zone.html`,
`#team-tab-zone`) : **l'onglet actif redevient un fait serveur**. Deux tests
neufs le gardent, dont un qui vérifie qu'aucun onglet ne vise plus le conteneur
interne — la première rédaction ne vérifiait que la présence d'un bon
`hx-target`, et survivait à la mutation.

L'administration de compétition résout la même chose par un `onclick` qui déplace
la classe côté client. Non repris : il ment si l'on clique l'onglet inerte
« Matchs », il dérive au retour arrière du navigateur, et la carte ne veut pas de
JS.

## Trois écarts à la carte, tous forcés par l'état du dépôt

**La portée CSS que la carte prescrit ferait échouer `check-arch`.** Elle dit
« portée par `.treasury` » ; l'axe 15 impose *point + nom du fichier*, donc
`pages/team-treasury.css` exige `.team-treasury`. Le nom du fichier fait foi — il
est dans le bundle et dans la spec — et `team-page.css` / `.team-page` posait
déjà l'idiome.

**La maquette s'appuie sur `--dark-7`, que la carte 448 a supprimé** l'avant-veille.
Ses deux usages passent au couple que la 448 a précisément établi : survol
`--dark-5`, fond doux `--dark-6`.

Conséquence à l'écran, vérifiée : la pastille de l'icône et le fond de la ligne
d'ouverture portaient alors **le même token**, et la pastille disparaissait. Le
défaut existait déjà dans la maquette — `--dark-6` contre `--dark-7`, 1,0012 de
rapport — la 448 n'a fait que le rendre inévitable. La ligne d'ouverture reçoit
donc une pastille blanche.

**Le détail des coups de pouce répète son propre libellé.** Le service rend
« Coups de pouce » comme détail *et* le builder comme libellé ; la maquette y
montrait la liste des coups de pouce achetés, que l'événement ne porte pas. Le
repli est écrit une fois pour tous les motifs — c'est la prochaine collision
qu'il attrape, pas seulement celle-ci.

## Deux morceaux de code défensif qu'aucun cas n'atteignait

La falsification a montré qu'un plancher d'indice sur la remontée n'était jamais
atteint : le filtre sur le motif borne déjà tout. Il est **supprimé** plutôt que
gardé sans test.

La clause « une ligne qui porte déjà un contexte ne remonte pas », elle, n'était
atteinte par aucune donnée actuelle — mais encode un invariant réel : elle est
**gardée**, et un test la rejoint en construisant une bourde coûteuse porteuse
d'un rapport, ce que le modèle permet même si les données ne le produisent pas
encore.

## Falsification

| Mutation | Constaté |
|---|---|
| L'encaissé ne retranche plus la dotation | `l_equation_du_bandeau_est_vraie` + 1 rouge |
| L'état vide ne se déclenche jamais | `la_dotation_seule_est_un_releve_vide` rouge |
| Le moins typographique devient un trait d'union | `le_montant_porte_son_signe…` rouge |
| Le détail n'est plus replié | 2 rouges |
| Une ligne sans contexte referme la période | 6 rouges |
| Chaque ligne à contexte ouvre une période | `deux_lignes_du_meme_match…` rouge |
| La bourde devient une correction | `les_huit_motifs…` rouge |
| Le mois est décalé d'un cran | `la_date_est_le_jour_et_le_mois` rouge |
| Le titre inverse journée et adversaire | 2 rouges |
| Le compte de mouvements exclut la dotation | 2 rouges |
| La remontée est supprimée | 2 rouges |
| La remontée absorbe tous les motifs | `un_recrutement_ne_se_fait_pas_absorber…` rouge |
| Une ligne déjà rattachée remonte quand même | `une_ligne_deja_rattachee…` rouge |
| Un plancher d'indice retiré | **passait** — code supprimé |
| Les classes d'icône crédit / débit interverties | `chaque_nature_de_ligne…` rouge |
| Le badge « Correction » posé partout | `seule_une_correction_porte_le_badge` rouge |
| La couleur du montant suit la nature | `la_couleur_du_montant…` rouge |
| L'état vide n'exclut plus le tableau | `l_etat_vide_remplace…` rouge |
| Un onglet revient viser le conteneur interne | **passait**, puis rouge après resserrement |
| Le conteneur de contenu disparaît de la zone | 2 rouges |
| « Trésorerie » n'est plus jamais actif | 2 rouges |

## Un test instable corrigé au passage

`test_un_nom_deja_pris_s_affiche_sous_le_champ` échouait dans la suite complète
et passait 5 fois sur 5 isolément : le clic de soumission tombait dans la fenêtre
de câblage htmx — le formulaire est visible avant d'être câblé, le clic se perd
sans laisser de trace, et l'attente expire sur un formulaire jamais soumis.
C'est le piège que le `CLAUDE.md` documente ; `attendre_cablage` le referme.

Il n'appartient pas à cette carte et fait l'objet d'un commit séparé.
