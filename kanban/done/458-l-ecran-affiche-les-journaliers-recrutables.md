# L'écran affiche les journaliers recrutables

**Épic :** E15 — Recruter un journalier
**Ordre :** 5 · **Dépend de :** 456, 457
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

### Le panneau appartient à `players`, pas à `teams`

**Le journalier vit dans `players`.** Son nom, ses SPP, la compétence qu'il a
gagnée au match : `players` les possède, et il les affiche déjà — la fiche
d'équipe compose son widget d'effectif sans rien savoir de son contenu
(`teams-squad-tab.html:12`, par `vm.players_widget_url`).

Le `CLAUDE.md` ne laisse pas le choix : *« L'assemblage de données issues de
plusieurs BCs se fait **exclusivement** au niveau du frontend, par composition
de widgets HTMX. »* Faire transiter un libellé d'amélioration dans un DTO de
port va contre cette règle — et c'est ce que la première version de cette carte
proposait.

`players` a d'ailleurs tout ce qu'il faut, y compris le tarif de base du poste :
`base_position_kpo(roster_line_id, catalog)` existe déjà dans
`player_creation.rs`. Il rend donc les quatre colonnes, **décomposition du prix
comprise**, sans que `teams` ne voie jamais une compétence.

### Ce que `players` ne peut pas décider

L'état du bouton. Ces trois refus appartiennent à `teams`, et lui seul les
connaît :

| | |
|---|---|
| effectif complet | le plafond de seize, dans sa définition de la carte 457 |
| trésorerie insuffisante | le total du panier |
| déjà dans le panier | le contenu du panier |

`teams` gagne donc `action_for_journeyman`, calqué sur `action_for_position`
(`recruitment_basket.rs:314`), qui rend déjà la cause exacte.

### L'hôte injecte l'état — trois paramètres

```
GET …/players/widgets/journeymen/{team_id}
    ?action_url=…/recruitment/journeyman/{player_id}
    &recrutables=<id>,<id>
    &motif=Effectif+complet
```

**`action_url` porte le gabarit du POST**, donc `players` ne connaît aucune
route de `teams`.

**Un seul motif suffit, et ce n'est pas une approximation.** Le plafond frappe
tout le monde ou personne ; s'il ne frappe pas, les seuls bloqués sont les trop
chers, et leur cause est la même. Les deux règles ne produisent jamais deux
motifs différents en même temps. Quant aux journaliers déjà au panier, ils ne
sont pas dans la liste — `hireable_journeymen()` les écarte.

### C'est le catalogue qui pose le conteneur, pas la page

Le panneau doit se rafraîchir sur `basketChanged` : recruter un journalier le
retire de la liste et change le budget. Or l'URL est figée au rendu (règle 4 des
widgets).

**Le catalogue se recharge déjà sur cet événement** : en posant le conteneur
lui-même, il reconstruit l'URL à chaque fois, avec l'état frais. Si la page le
posait, le panneau garderait l'état du chargement initial.

L'URL est construite dans le view model par `AppRoutes`, jamais par un import
direct des routes de `players` — comme `TeamDetailVm` le fait déjà.

### `improvement_label` sort de `SquadMemberDto`

La carte 454 l'y avait mis pour que `teams` l'affiche. Il n'a plus lieu d'être,
et son départ règle un défaut au passage : c'était un **libellé pré-écrit dans
un adapter**, dont la règle — « la première compétence acquise » — n'est juste
que parce qu'un journalier n'a pas de passé. Pour un joueur ordinaire, il aurait
nommé une compétence prise six matchs plus tôt, sans que son nom le laisse
deviner.

`is_temporary` reste : `teams` en a besoin pour son plafond.

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

### Les deux routes

```
GET  …/players/widgets/journeymen/{team_id}   → le panneau, rendu par players
POST …/recruitment/journeyman/{player_id}     → post_add_journeyman, dans teams
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

Côté `players`, sur le view model du widget :

| Test | Ce qu'il prouve |
|---|---|
| `un_journalier_sans_amelioration_rend_none` | « aucune », pas une chaîne vide |
| `le_prix_se_decompose_au_dela_du_tarif` | `price > base` |
| `le_prix_ne_se_decompose_pas_a_l_egalite` | pas de « 65 + 0 » |
| `la_liste_est_vide_sans_journalier` | le panneau se masque |
| `un_journalier_non_recrutable_porte_le_motif` | l'état injecté par l'hôte |

Côté `teams`, sur le panier :

| Test | Ce qu'il prouve |
|---|---|
| `action_for_journeyman_rend_la_cause_exacte` | le pendant de `action_for_position` |

Les tests de navigateur sont la carte 459.

## Checklist

- [x] `players` : le widget, son view model, sa route
- [x] `teams` : `action_for_journeyman` sur le panier
- [x] Le catalogue pose le conteneur, URL par `AppRoutes`, refaite à chaque rendu
- [x] La route du POST, le handler, `HX-Trigger: basketChanged`
- [x] `improvement_label` retiré de `SquadMemberDto` et de `squad_adapter`
- [x] Les styles dans `rec-page.css`, portés par `.rec-page`
- [x] Huit tests — six prévus, deux de plus
- [x] `make lint`, `make check-arch`, `make test` — 1701 tests
- [x] `make e2e` — 356 passés, 7 ignorés
- [x] `make audit` — une dépendance a été ajoutée

## Ce qui a été fait

### Une dépendance ajoutée : `urlencoding`

Les motifs de refus sont du texte français — espaces, accents, deux-points — et
l'hôte les passe en paramètre de requête. La crate était déjà dans l'arbre par
`axum-login` ; la déclarer en direct est ce qui autorise à s'en servir.
`make audit` passe.

### Le CSS est porté par `.rec-page`

Le nom du fichier **est** le sélecteur de portée : `rec-page.css` ⇒ toute règle
sous `.rec-page`. Les sélecteurs du panneau ont dû être préfixés, sans quoi
l'axe 17 les aurait refusés — une feuille dont les règles ne rencontrent jamais
leur markup.

### Le `player_id` n'est pas gardé par `space_scope`, et n'a pas à l'être

Il n'a pas de résolveur, donc le middleware le laisse passer. Ce n'est pas un
trou : le panier ne le trouve que dans les recrutables **de cette équipe**, et
un identifiant étranger rend `JourneymanNoLongerAvailable`. La portée est tenue
par la donnée, pas par un contrôle ajouté — et `action_for_journeyman` rend la
même cause à l'affichage, donc l'écran et l'écriture disent la même chose.
