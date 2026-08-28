# L'écran affiche les journaliers recrutables

**Ordre :** 4 · **Dépend de :** 456, 457
**Conception :** `docs/specs/embaucher-un-journalier/ecran-de-recrutement/`
(`02-front.md`, `04-dtos.md`) · **Maquette :**
`assets/rawpages/html/app-team-recruitment.html`

## Objectif

Le panneau qui rend la décision possible.

## Pourquoi elle dépend aussi de la 456

Sans la disparition, l'écran afficherait des journaliers de **matchs anciens**
qui ne devraient plus être là — un panneau qui grossit à chaque match et ne se
vide jamais.

## Conception

### Aucun widget nouveau

`teams-recruitment.html` est **déjà** un assemblage à deux widgets qui se
resynchronisent sur `basketChanged`. La section entre dans le **catalogue**, qui
se recharge déjà — donc recruter un journalier le retire de la liste sans un
mécanisme de plus.

Un troisième widget coûterait cher pour rien : le budget est commun, la limite
de 16 est commune, et il faudrait un abonnement de plus au même événement pour
afficher une donnée que le premier a déjà chargée.

### Le view model

```rust
pub struct JourneymanRowVm {
    pub player_id: String,
    pub name: String,
    pub position_name: String,
    pub spp: u32,
    pub improvement: Option<String>,   // None → « aucune »
    pub price_kpo: u32,
    pub base_price_kpo: u32,           // pour décomposer
    pub action: ActionVm,
}

pub struct RecruitmentCatalogVm {
    …,
    pub journeymen: Vec<JourneymanRowVm>,   // vide si aucun
}
```

**`ActionVm` est réutilisé tel quel** — il porte déjà `Enabled`, `Blocked`,
`Forbidden`, et son `from_domain` traduit une `ActionState`. La ligne de
journalier n'invente rien : elle affiche « n'est plus disponible » exactement
comme les autres affichent « quota atteint ».

**Vide et non `Option`** : le gabarit teste `is_empty()`, et une collection vide
dit la même chose qu'une absence sans obliger à déballer.

**Construit dans `builders.rs`**, pas par `from_domain` : une ligne de journalier
dépend du panier **et** du DTO de port — le nom, les SPP et l'amélioration
viennent de `SquadMemberDto`. C'est la règle du `CLAUDE.md`.

### Le panneau

Au-dessus de « Recruter un joueur », rendu sous condition.

**Filet ambré et avertissement en clair** : *« Ils partent à la fin de cette
phase. Un journalier qui n'est pas recruté maintenant est perdu — avec son
expérience. »*

C'est la seule différence de nature entre ce panneau et le catalogue, et elle
doit **se voir sans être lue** : un poste sera encore là au prochain match.

**Le panneau disparaît quand la liste est vide.** La plupart des matchs se
jouent sans journalier, et un panneau vide poserait la question « qu'est-ce que
j'ai raté ? » à chaque phase de recrutement.

### Les quatre colonnes

| Colonne | Contenu |
|---|---|
| Journalier | le nom, le poste en dessous |
| Expérience | « 6 PSP », grisé à zéro |
| Amélioration | la compétence, ou « aucune » |
| Prix | la valeur courante, **décomposée** si elle dépasse le tarif du poste |

**La décomposition n'est pas un ornement** : « 65 + 20 d'amélioration ». Sans
elle, un coach qui voit 85 pour un Trois-quart à 65 croit à une erreur. C'est la
règle du LRB rendue lisible à l'endroit où elle s'applique.

Le calcul de l'écart est fait **au rendu** — c'est une soustraction, pas une
donnée.

### La route et le handler

```
POST …/recruitment/journeyman/{player_id}   → post_add_journeyman
```

**Aucun corps** : il n'y a rien à choisir, ce journalier-là ou aucun.

**`{player_id}` dans le chemin**, jamais dans le corps — la leçon de la carte
416. Il n'est pas résolu par `space_scope`, mais il n'a pas besoin de l'être :
le use case ne le trouve que dans la liste des recrutables **de cette équipe**,
et un identifiant étranger donne `JourneymanNoLongerAvailable`. **La portée est
tenue par la donnée**, pas par un contrôle ajouté.

| Cas | Réponse |
|---|---|
| ajouté | `HX-Trigger: basketChanged` |
| refus du domaine | `422` + le catalogue re-rendu |
| conflit de version | le conflit existant du panier |

### CSS

**Aucune feuille neuve.** Les styles vont dans `widgets/rec-page.css`, déjà au
bundle — rien à inscrire dans `css_bundle.rs`.

## Tests

Unitaires, sur `builders.rs` :

| Test | Ce qu'il prouve |
|---|---|
| `un_journalier_sans_amelioration_rend_none` | « aucune », pas une chaîne vide |
| `le_prix_se_decompose_au_dela_du_tarif` | `price > base` |
| `le_prix_ne_se_decompose_pas_a_l_egalite` | pas de « 65 + 0 » |
| `la_liste_est_vide_sans_journalier` | le panneau se masque |

Les tests de navigateur sont la carte 459.

## Checklist

- [ ] `JourneymanRowVm` et le champ du catalogue
- [ ] `builders.rs` — construit depuis le panier **et** le DTO de port
- [ ] Le panneau, rendu sous condition, au-dessus du catalogue
- [ ] La route, le handler, `HX-Trigger: basketChanged`
- [ ] Les styles dans `rec-page.css`
- [ ] Les quatre tests
- [ ] `make lint && make test && make check-arch`
