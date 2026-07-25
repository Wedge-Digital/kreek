# Schéma des données de référence

kreek est un moteur de gestion de ligue : il ne contient aucune règle de jeu en
dur, il **lit un ruleset** au démarrage. Ce document décrit le format attendu.

Le répertoire est désigné par la variable d'environnement `REFERENCES__DIR`
(défaut : `assets/references`, cf. `config/default.toml`). Un jeu de
démonstration complet et fonctionnel est fourni dans `assets/references.example/`
— c'est la spécification exécutable de ce document.

## Règles générales

- **Neuf fichiers sont obligatoires**, aux noms exacts listés ci-dessous. Un
  fichier manquant ou mal formé fait échouer le démarrage avec un message
  nommant le fichier fautif.
- Le suffixe `_fr` des noms de fichiers est historique : il n'existe aucun
  mécanisme de bascule de langue, ces noms sont figés dans le chargeur.
- **Les champs inconnus sont ignorés** silencieusement. Un ruleset peut porter
  des métadonnées supplémentaires (`ruleset`, `edition`, notes…) sans risque —
  mais une faute de frappe sur un nom de champ passe donc inaperçue.
- Tous les montants sont exprimés dans l'unité de trésorerie du jeu (kPo).

| Fichier | Racine JSON | Contenu |
|---|---|---|
| `teams_fr.json` | `{ "teams": [...] }` | rosters et postes jouables |
| `skills_fr.json` | `{ "skills": [...] }` | compétences |
| `skill_cat_fr.json` | `{ "skill_categories": [...] }` | catégories de compétences |
| `skill_cost.json` | `[...]` (tableau racine) | barème de progression |
| `star_players_fr.json` | `{ "star_players": [...] }` | joueurs vedettes |
| `inducements_fr.json` | `{ "inducements": [...] }` | coups de pouce |
| `staff_fr.json` | `{ "staff": [...] }` | personnel d'équipe |
| `special_rules_fr.json` | `{ "special_rules": [...] }` | règles spéciales de roster |
| `leagues_fr.json` | `{ "leagues": [...] }` | ligues d'origine |

`spp_rules.json` est également présent dans le jeu d'exemple mais **n'est pas
chargé** par le moteur : c'est une préparation pour une évolution à venir.

---

## `teams_fr.json`

| Champ | Type | Requis | Sémantique |
|---|---|---|---|
| `uid` | string | oui | identifiant du roster, unique |
| `name` | string | oui | libellé affiché |
| `rerollCost` | entier | oui | prix d'une relance d'équipe |
| `tier` | string | oui | libellé libre de palier |
| `specialRules` | string[] | non (défaut `[]`) | uids de `special_rules_fr.json` |
| `allowedStaff` | string[] | non (défaut `[]`) | uids de `staff_fr.json` |
| `availablePlayers` | objet[] | non (défaut `[]`) | postes, voir ci-dessous |
| `leagues` | string[] | **de fait requis** | uids de `leagues_fr.json` — voir ci-dessous |
| `logo` | string \| null | non | nom de fichier d'illustration |

### Poste (`availablePlayers[]`)

| Champ | Type | Requis | Sémantique |
|---|---|---|---|
| `uid` | string | oui | identifiant du poste, unique globalement |
| `positionName` | string | oui | libellé affiché |
| `cost` | entier | oui | prix d'achat |
| `MA` `ST` `AG` `PA` `AV` | **entiers** | oui | caractéristiques |
| `skills` | string[] | non (défaut `[]`) | uids de `skills_fr.json` |
| `primaryAccess` | string[] | non (défaut `[]`) | ids de `skill_cat_fr.json` |
| `secondaryAccess` | string[] | non (défaut `[]`) | ids de `skill_cat_fr.json` |
| `max_quantity` | entier | oui | plafond par équipe (noté en snake_case, contrairement aux autres champs) |
| `is_journeyman` | booléen | non (défaut `false`) | poste servant de remplaçant temporaire |

> **`leagues` est optionnel pour le parseur, obligatoire en pratique.** Une
> équipe ne peut pas être soumise sans ligue, et la ligue n'est assignée
> automatiquement que si le roster en déclare **exactement une**. Un roster qui
> n'en déclare aucune produit des équipes impossibles à soumettre ; un roster qui
> en déclare plusieurs impose un choix explicite à l'utilisateur.

---

## `star_players_fr.json`

| Champ | Type | Requis | Sémantique |
|---|---|---|---|
| `uid` `name` | string | oui | identifiant et libellé |
| `cost` | entier | oui | prix de recrutement |
| `MA` `ST` | **entiers** | oui | mouvement et force |
| `AG` `PA` `AV` | **chaînes** | oui | ex. `"3+"`, `"-"` pour « sans objet » |
| `playerType` | string | oui | libellé libre (rôle, origine) |
| `skills` | string[] | non (défaut `[]`) | uids de `skills_fr.json` |
| `specialAbilityName` | string | oui | nom de la capacité propre |
| `specialAbilityDescription` | string | oui | texte affiché |
| `playsFor` | string[] | non (défaut `[]`) | uids de ligues ; vide = aucune attache |
| `availableForRosters` | string[] | non (défaut `[]`) | uids de rosters pouvant le recruter |

> **Piège** : `AG`, `PA` et `AV` sont des chaînes ici, alors que les mêmes
> caractéristiques sont des entiers sur les postes de `teams_fr.json`. C'est
> volontaire — un star player peut porter `"-"`, un poste jamais.

---

## `skills_fr.json`

| Champ | Type | Requis | Sémantique |
|---|---|---|---|
| `uid` `name` | string | oui | identifiant et libellé |
| `category` | string | oui | id de `skill_cat_fr.json` |
| `type` | string | oui | palier de rareté (`Standard`, `Élite`…) — influence le barème |
| `activation` | string | oui | libellé libre (`Active`, `Passive`) |
| `description` | string | oui | texte affiché |

## `skill_cat_fr.json`

`id` et `label`, tous deux requis. Voir la section « Identifiants attendus ».

## `special_rules_fr.json` et `leagues_fr.json`

Même forme : `uid` et `label`, tous deux requis.

## `staff_fr.json`

`uid`, `name`, `price`, `maxQuantity`, `description` — tous requis.

---

## `skill_cost.json`

Tableau racine, un objet par niveau de progression.

| Champ | Type | Requis | Sémantique |
|---|---|---|---|
| `level` | entier | oui | niveau atteint |
| `chosen` | `{ primary, secondary }` | oui | coût d'une compétence choisie |
| `chosenElite` | `{ primary, secondary }` | non | variante pour les compétences `Élite` |
| `random` | entier | oui | coût d'une compétence tirée au hasard |
| `randomElite` | entier | non | variante pour les compétences `Élite` |
| `characteristic` | entier | oui | coût d'une amélioration de caractéristique |

Si `chosenElite` / `randomElite` sont absents, le moteur retombe sur `chosen` /
`random`. Le jeu d'exemple renseigne ces champs sur les trois premiers niveaux
et les omet sur les suivants, pour documenter les deux comportements.

---

## `inducements_fr.json`

| Champ | Type | Requis | Sémantique |
|---|---|---|---|
| `uid` `name` | string | oui | identifiant et libellé |
| `cost` | entier | oui | prix plein |
| `reducedCost` | entier \| null | oui | prix réduit, `null` si sans objet |
| `maxQuantity` | entier | oui | plafond par rencontre |
| `category` | string | oui | voir « Identifiants attendus » |
| `restrictedTo` | string[] | non (défaut `[]`) | uids de rosters ; vide = ouvert à tous |
| `description` | string | oui | texte affiché |

---

## Identifiants attendus par le moteur

Certains identifiants sont interrogés **en dur** dans le code. Un ruleset qui ne
les fournit pas reste chargeable, mais dégrade silencieusement des
fonctionnalités.

| Identifiants | Fichier | Ce qui casse en leur absence |
|---|---|---|
| `APOTHECARY`, `CHEERLEADERS`, `COACH_ASSISTANTS`, `FAN_FACTOR` | `staff_fr.json` | typage du staff, achat en construction d'équipe |
| `GENERAL`, `AGILITY`, `STRENGTH`, `PASSING` | `skill_cat_fr.json` | classement et style des catégories dans le sélecteur |
| `FAVOURED_OF_KHORNE`, `FAVOURED_OF_NURGLE`, `FAVOURED_OF_SLAANESH`, `FAVOURED_OF_TZEENTCH`, `FAVOURED_OF_UNDIVIDED` | `special_rules_fr.json` | sélecteur de règle à choix (voir ci-dessous) |
| `COMMON`, `INFAMOUS_STAFF`, `WIZARD`, `BIASED_REFEREE` | champ `category` des inducements | regroupement « commun / spécifique » |

**Règles spéciales à choix** : si un roster porte une règle dont l'uid commence
par `FAVOURED_OF_CHOOSE_`, le moteur propose à l'utilisateur de choisir parmi les
cinq uids `FAVOURED_OF_*` ci-dessus. Les **libellés** sont libres — seuls les
uids sont contraints.

### Quirks connus

- Le moteur associe un style à la catégorie `MUTATION` (au singulier) alors que
  les jeux de données existants utilisent `MUTATIONS`. Les catégories `DEVIOUS`
  et `TRAITS` n'ont pas non plus de style associé. Sans conséquence
  fonctionnelle : le rendu retombe sur le style par défaut.

---

## Cohérence référentielle

Le parsing ne vérifie que la forme. La cohérence entre fichiers — « l'uid cité
existe-t-il ? » — est contrôlée par
`app::references::domain::consistency::check_consistency`, qui vérifie que :

- chaque compétence citée par un poste ou un star player existe ;
- chaque catégorie citée en accès primaire ou secondaire existe ;
- chaque staff de `allowedStaff` existe ;
- chaque règle spéciale d'un roster existe ;
- chaque roster de `availableForRosters` existe.

Le jeu d'exemple est vérifié par ce contrôle à chaque exécution de la suite de
tests. Un uid orphelin n'empêche pas le démarrage : l'entrée concernée est
simplement introuvable à l'affichage.
