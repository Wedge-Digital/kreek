# BC `players` — Widget de customisation et bascule de la fiche

**Priorité : haute**
**Dépend de :** `304-players-customisation-basket-domain.md`
**Contexte :** `players` — widget HTMX + contrôleur de page

## Objectif

Tout le chemin de **lecture** : le panneau, l'autorisation resserrée, et le
choix de l'occupant du slot à l'ouverture de la fiche. Les trois se testent
ensemble — un widget qu'on ne peut pas atteindre ne prouve rien.

**Spec :** `02-front.md` et `04-dtos.md`.
**Maquette :** `assets/rawpages/html/app-player-detail-readonly.html`, mode
`customise`.

---

## Le troisième occupant du slot

`#pd-right-panel` accueille déjà le journal des évolutions et le panneau de
dépense de SPP, qui s'y remplacent en `outerHTML` sur le même id. La
customisation en devient le troisième.

Répéter l'id du conteneur est ici **correct** : en `outerHTML` le fragment
*est* le conteneur. L'interdiction du CLAUDE.md vise les swaps `innerHTML`.

## L'autorisation se resserre

`can_customise` s'appuie aujourd'hui sur `check_admin_rights`, qui inclut **le
coach de l'équipe**. La phase 1 l'exclut : commissaire de ligue et admin
d'espace uniquement.

**La valeur change, pas le type.** Rien ne cassera à la compilation, un coach
perdra simplement un droit qu'il avait. C'est un changement que seul un test
peut attraper — d'où son inscription explicite en checklist.

## Le mode se déduit du panier

Le contrôleur de page choisit l'occupant du slot : **panier existant *et* droit
de customiser**, jamais l'un sans l'autre. Sans la seconde condition, un panier
laissé ouvert ferait apparaître le mode administration à un coach.

Un panier de plus de 24 h est **supprimé** au passage, et le journal affiche un
message discret — « votre saisie de plus de 24 h a été abandonnée ». C'est un
`GET` qui écrit, ce qui est assumé : la suppression est idempotente.

## Le template

Markup et CSS repris de la maquette. **Le JS ne l'est pas** : il simulait un
panier client qui n'existe pas. Ce qui reste en Alpine se réduit à la bascule
d'onglets et au filtre de recherche ; tout le reste est rendu par le serveur —
valeurs effectives, aperçus, lignes en attente, grisage des boutons.

Les champs de caractéristique sont des **chaînes déjà formatées** : le suffixe
des seuils de dé et le sens de l'offset dépendent de la caractéristique, et les
résoudre dans le template y remettrait la table de directions du domaine.

## Port à étendre

`ISkillCatalogPort` sait résoudre **une** compétence, pas les lister. L'onglet
compétences a besoin du catalogue complet, non filtré par l'accès du poste —
la customisation ignore les règles du jeu par définition.

---

## Checklist

- [ ] `ISkillCatalogPort::list_all_skills()` + adapter
- [ ] `player_customisation_widget` — crée le panier s'il n'existe pas
- [ ] Template repris de la maquette (markup et CSS)
- [ ] `CustomisationVm` et ses enfants
- [ ] `can_customise` resserré — plus le coach de l'équipe
- [ ] Choix de l'occupant du slot : panier **et** droit
- [ ] Péremption 24 h vérifiée à l'ouverture, avec message d'abandon
- [ ] Bouton « ✎ Customiser » : `disabled` retiré, `hx-get` branché
- [ ] Route + wiring
- [ ] **Test : un coach d'équipe ne voit pas le bouton** — le changement
      silencieux de `can_customise`
- [ ] Test : un panier existant rouvre le mode pour un commissaire
- [ ] Test : le même panier laisse la fiche classique à un coach
- [ ] Vérification navigateur : rendu conforme à la maquette
