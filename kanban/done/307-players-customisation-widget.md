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

## L'autorisation est déjà bonne — mais non gardée

`can_customise` s'appuie sur `check_admin_rights`, qui vérifie admin d'espace
puis admin de compétition, **sans** le coach. C'est exactement ce que veut la
phase 1 : rien à resserrer.

La spec affirmait le contraire, par confusion avec `can_spend_spp` — lui
explicitement « étendu au coach ». Corrigé.

Reste que la règle ne tient qu'à la composition d'une fonction que rien
n'empêche d'élargir, et qu'aucun test ne la garde. C'est ce test qu'il faut
écrire.

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

- [x] `ISkillCatalogPort::list_all_skills()` + adapter — **livré en carte 306**, l'hydratation en dépendait
- [x] `player_customisation_widget` — crée le panier s'il n'existe pas
- [x] Template repris de la maquette (markup et CSS)
- [x] `CustomisationVm` et ses enfants
- [x] `can_customise` — **déjà correct**, aucun changement (voir ci-dessus)
- [x] Choix de l'occupant du slot : panier **et** droit
- [x] Péremption 24 h vérifiée à l'ouverture, avec message d'abandon
- [x] Bouton « ✎ Customiser » : `disabled` retiré, `hx-get` branché — et **masqué** pour qui n'a pas le droit
- [x] Route + wiring — panier injecté dans `PlayersContext`, les sept `POST` déclarés mais branchés en 308
- [x] **Test : un coach d'équipe ne voit pas le bouton** — garde-fou d'un
      comportement existant que rien ne protège aujourd'hui
- [x] Test : un panier existant rouvre le mode pour un commissaire
- [x] Test : le même panier laisse la fiche classique à un coach
- [x] Vérification du rendu : test `le_panneau_rend_ses_quatre_onglets_et_son_panier`
      + aperçu autonome hors serveur (le serveur local n'a pas d'utilisateur
      `legacy_id = 1`, donc `BYPASS_AUTH` ne connecte personne)

---

## Écarts assumés à la spec, à connaître pour la 308

**Pas d'amplitude libre sur les caractéristiques.** La maquette proposait un
champ numérique, puis « Améliorer » / « Dégrader ». Le contrat de la phase 4
(`can_improve`, `can_degrade`, `preview_up`, `preview_down`) décrit un cran par
clic : ce sont des booléens et des chaînes uniques, pas des fonctions de
l'amplitude. Un aperçu et un grisage rendus par le serveur ne peuvent pas
suivre une saisie client. Deux crans se font donc en deux clics, et laissent
deux lignes au panier — ce qui reste conforme à « les amplitudes ne concernent
qu'une seule modification à la fois ».

**`refusal` n'est pas dans le VM.** Rien ne le produit tant que les `POST`
n'existent pas ; la 308 l'ajoute avec eux.

**Les sept `POST` sont déclarés, pas branchés.** Le panneau rend leurs URLs, le
routeur ne les connaît pas encore : un clic sur une action donne un `404` tant
que la 308 n'a pas atterri. La 307 se vérifie au rendu, pas au clic.

**Les formulaires portent `expected_version`.** La garde d'écriture concurrente
de `customisation_basket_mutation` l'exige ; les DTOs de la 308 doivent donc
tous le déclarer, ce que la phase 4 avait omis.

## Ce que le rendu a révélé

La table `category` → classe CSS, copiée de `skill_picker.rs`, disait
`MUTATION` là où le référentiel dit `MUTATIONS`, et ignorait `DEVIOUS` et
`TRAITS`. **Trois catégories sur sept** prenaient donc la pastille « général »
sans que rien ne le signale.

Corrigé ici, avec `try_category_css` qui distingue « connue et générale » de
« inconnue » — sans quoi aucun test ne peut voir le repli — et un test qui lie
la table au référentiel réel.

**Le défaut subsiste dans `references::skill_picker`, `player_table_widget` et
`team_created_listener`**, qui portent la même table. Hors périmètre de cette
carte ; à ouvrir.
