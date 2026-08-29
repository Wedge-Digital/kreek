# Les tests E2E de l'onglet Paramètres

**Épic :** E14 · **Ordre :** 4 · **Dépend de :** 421 à 425
**Conception :** `docs/specs/modifier-une-competition/onglet-parametres/07-integration.md`

## Objectif

Prouver dans un navigateur ce qu'aucun test unitaire ne peut voir : le rendu
HTMX, la collecte JS du picker, le recalcul bout en bout, et les effacements
silencieux.

Fichier : `tests/e2e/test_competition_admin_settings.py`.

## Les scénarios

| Test | Ce qu'il prouve |
|---|---|
| `test_onglet_parametres_charge_les_cinq_panneaux` | l'assemblage et les cinq `hx-get` |
| `test_renommer_la_competition` | le POST le plus simple, bout en bout |
| `test_nom_deja_pris_affiche_l_erreur_sous_le_champ` | l'emplacement d'erreur |
| `test_modifier_le_bareme_recalcule_le_classement` | **le scénario central** |
| `test_retirer_une_poule_desaffecte_ses_equipes` | la cascade, vue dans l'onglet Poules |
| `test_retirer_toutes_les_poules` | le cas que la projection paresseuse traite le plus mal |
| `test_modifier_les_coups_de_pouce_d_un_tier` | la collecte JS de l'événement du picker |
| `test_le_calendrier_survit_a_l_enregistrement_des_poules` | la relecture du JSONB |
| `test_les_coachs_invites_survivent_au_changement_de_visibilite` | idem, autre document |
| `test_un_non_admin_est_refuse_sur_les_onze_routes` | **paramétré**, `403` sur chaque GET et chaque POST |
| `test_un_admin_de_competition_ouvre_les_cinq_panneaux` | admin nommé, sans être admin d'espace |
| `test_un_admin_d_espace_ouvre_les_cinq_panneaux` | admin d'espace, sans être nommé |

## Les quatre qui valent le prix de la suite

**`test_modifier_le_bareme_recalcule_le_classement`** — deux matchs joués,
victoire portée de 2 à 3 points, le classement affiche le nouveau total. Le
recalcul ne se vérifie pas unitairement de bout en bout : entre le POST et la
ligne affichée, il y a un port, un adapter, un use case d'un autre BC et une
transaction.

**`test_le_calendrier_survit_a_l_enregistrement_des_poules`** — son échec ne
produirait aucune erreur, juste un calendrier vide découvert des jours plus
tard. C'est le défaut le plus silencieux de tout l'onglet.

**`test_modifier_les_coups_de_pouce_d_un_tier`** — le picker n'a pas de champ
caché. Sans la collecte JS, le POST part avec des listes vides et
l'enregistrement réussit. Aucun test unitaire ne peut voir ça.

**`test_un_non_admin_est_refuse_sur_les_onze_routes`** — paramétré sur les onze,
GET et POST. Masquer l'onglet est du confort : les URL restent atteignables, ce
sont elles qui refusent.

## Les deux cas positifs d'autorisation sont distincts

`require_admin_access` accepte par deux chemins différents — `SpaceProfile::SpaceAdmin`,
ou l'appartenance à `admin_ids`/`admin_names` de la compétition. Un seul test les
confondrait, et une régression sur l'un passerait inaperçue.

## Le piège de la fenêtre non câblée

Les cinq widgets arrivent par `hx-get`. Ils sont donc exactement dans la fenêtre
où un élément est peint, cliquable, et **inerte** — HTMX câble le contenu
inséré quelques dizaines de millisecondes après l'avoir rendu visible.

```python
from htmx_helpers import cliquer_quand_cable
cliquer_quand_cable(page, "#settings-ranking button[type=submit]")
```

**Pas de `sleep`.** Une durée fixe n'a aucune marge sur une machine chargée —
c'est exactement là que la suite échouait — tout en coûtant son délai aux
milliers d'appels où tout est déjà prêt.

## Checklist

- [x] Les douze scénarios — **neuf existaient déjà**, cf. ci-dessous
- [x] Aucun `sleep`
- [x] `make e2e` vert, serveur de développement lancé par l'utilisateur

## Ce qui a été livré, et pourquoi ce n'est pas la liste de la carte

Les cartes 421 à 425 ont livré leur e2e **au fil de l'eau**, chacune dans son
fichier. Neuf des douze scénarios existaient donc avant d'ouvrir cette carte :
renommage, nom déjà pris, barème → recalcul, poule retirée, toutes les poules
retirées, coups de pouce, calendrier préservé, invités préservés, et un test de
garde par panneau.

Les rejouer dans un fichier unique aurait coûté une dizaine de minutes de
`make e2e` **pour re-prouver l'acquis**. Ce fichier porte à la place les trois
choses transverses qu'aucun fichier de panneau ne pouvait porter seul.

### 1. Les onze routes, paramétrées

Le compte est exact : l'onglet en `GET`, plus cinq panneaux × (`GET` + `POST`).
Un test de garde par fichier couvre son panneau ; aucun ne couvre **la douzième
route que quelqu'un ajoutera**. Le trou réel était `settings/general`, gardé
dans le code mais asserté nulle part.

### 2. Les deux chemins d'autorisation, séparés

`require_admin_access` accepte par `is_space_admin || is_comp_admin`. Or dans
l'espace e2e, **`DevCoach` est les deux à la fois** — `SpaceAdmin` dans
`spaces__user_space`, `CompetitionAdmin` dans `competitions_members`. Tous les
tests positifs existants franchissaient donc les deux portes ensemble.

L'isolation se fait avec les identités réelles : `E2E Coach 01` (qui est
`SpaceUser`) inscrit sur la compétition pour le premier chemin, `DevCoach`
retiré des membres pour le second. Chacun restaure son montage en `finally`.

### 3. L'assemblage qui se remplit

Le test de la carte 420 vérifie que les cinq **conteneurs** existent ; il a été
écrit quand ils étaient vides. Vérifié en falsifiant : avec un `hx-trigger`
cassé, il reste **vert** alors qu'un panneau ne se charge plus. Le nouveau test
attend `#settings-<nom>-panel`, produit par le widget et non par la page.

## Le piège rencontré : le corps est extrait avant la garde

Les extracteurs de corps d'axum s'exécutent **avant** le corps du handler, donc
avant `require_admin_access`. Un `POST` au corps vide rend `415`, jamais `403` :
mesuré sur les cinq panneaux avec un `data={}`.

Un test d'autorisation qui n'assertait que « ce n'est pas 200 » serait donc
**creux** — il constaterait un refus de format en croyant voir un refus de
droit. Chaque route paramétrée porte un corps analysable, et l'assertion exige
**exactement** `403`.

## Falsification

| Mutation | Constaté |
|---|---|
| Branche `is_comp_admin` retirée | **1 seul** test rouge — l'isolé ; les 14 autres verts |
| Branche `is_space_admin` retirée | **1 seul** test rouge — l'autre isolé |
| Garde retirée sur `POST /general` | le test paramétré rouge sur `general` seul, `200` au lieu de `403` |
| `hx-trigger` du conteneur des poules cassé | test d'assemblage rouge ; l'ancien test de la 420 **reste vert** |

Les deux premières lignes sont la démonstration de l'argument de la carte : sans
ces tests, supprimer l'une des deux branches d'autorisation ne se serait vu
**nulle part**.

## Une observation, hors périmètre

`find_competition_by_id.sql` ne filtre pas sur `competition_profile` :
`admin_ids` et `admin_names` contiennent **tous** les membres de la compétition,
quel que soit leur profil. Aujourd'hui sans effet — la table ne contient que des
`CompetitionAdmin` (3486 lignes, aucune compétition à plus d'un membre) — mais
le jour où un participant y recevrait une ligne, il deviendrait commissaire.
Non corrigé ici : ce serait élargir la carte sans l'avoir discuté.
