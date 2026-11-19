# Modifier une compétition la remet en construction

**Priorité : haute** — un clic met une compétition vivante hors service
**Dépend de :** rien · **Sans épic**
**Trouvée par :** l'utilisateur, sur le panneau de modification de l'administration

## Le symptôme

Enregistrer un réglage depuis l'administration fait régresser la saison sous
`ready`. La compétition redevient « en cours de construction ».

## La cause

`sql/seasons/update_rules.sql` réécrit le statut :

```sql
UPDATE competition_seasons
SET    name = $1, rules = $2::jsonb,
       status = 'rules_selected'      -- ← ici
WHERE  id = $3
```

C'est juste pendant la création — l'étape 2 du magicien fait avancer la saison
d'un cran. Ce ne l'est pas sur une saison en cours.

## L'inventaire, fait avant de corriger

Toutes les écritures atteignables depuis l'administration, remontées depuis le
SQL plutôt que devinées depuis les écrans :

| Onglet | Écriture | Régresse le statut |
|---|---|---|
| Paramètres · **Général** | `save_rules` + `update_base_info` | **oui** → `rules_selected` |
| Paramètres · **Classement** | `save_rules` | **oui** |
| Paramètres · **Tiers** | `save_rules` | **oui** |
| Paramètres · Poules | `save_structure_and_prune_groups` | non — corrigé carte 423 |
| Paramètres · Visibilité | `save_visibility` | non — corrigé carte 426 |
| Poules · tirage, reset, affectation | `save_assignments` | non |
| Calendrier · journées, appariements | `save_match_day`, `save_pairing`, `ensure_match_days_from_structure` | non |
| Inscriptions | routes du BC `teams` | non |

Les trois autres méthodes qui écrivent un statut — `save_structure`,
`save_invitations`, `set_ready` — ne sont appelées que par le magicien.
**La fuite est exactement `save_rules`, et elle seule.**

## Les conséquences, mesurées

Un `POST` sur le panneau Classement d'une saison `ready` :

```
statut avant : ready
statut après : rules_selected
```

**La carte de la compétition ne mène plus à la compétition.** Dans la liste, elle
pointe vers `/competitions/create/…/structure` — l'étape 2 du magicien.

**L'inscription est fermée.** Créer une équipe répond « Cette compétition n'est
pas encore ouverte aux inscriptions » — le garde-fou de la carte 407, qui fait
son travail sur une donnée devenue fausse.

En production le 2026-08-31 : les 9 saisons sont `ready`. **Aucune victime** — le
défaut est armé, pas encore déclenché.

## Pourquoi c'est passé deux fois avant d'être vu

Le piège a été rencontré **deux fois** et corrigé **deux fois**, chaque fois pour
le seul panneau où on l'avait vu :

- carte 423 → `update_structure_keep_status.sql`, pour les Poules ;
- carte 426 → `save_visibility` qui ne touche pas la colonne, pour la Visibilité.

Les deux portent un commentaire qui décrit exactement le défaut d'aujourd'hui.
**Personne n'a demandé si `save_rules` avait le même problème.**

Et un seul test le gardait : `test_competition_settings_pools.py` vérifie que la
saison reste `ready`. Les autres panneaux n'avaient pas cette assertion.

## La correction

**`update_rules_keep_status.sql`** — le même `UPDATE` sans la ligne de statut —
et `save_rules_keep_status` sur le port, que les trois panneaux appellent. Le
magicien garde `save_rules`.

C'est la forme déjà employée deux fois ; en inventer une troisième aurait rendu
le prochain lecteur incapable de reconnaître le motif.

## La généralisation : un verrou, pas des assertions recopiées

Copier « la saison reste `ready` » dans cinq fichiers e2e garderait les cinq
panneaux d'aujourd'hui et **aucun de demain**. C'est exactement ce qui a échoué :
la 423 avait posé l'assertion pour son panneau, et le défaut est revenu ailleurs
six semaines plus tard.

**Axe 16 de `check-arch`** : aucun use case de `settings/` n'appelle une méthode
qui écrit le statut de la saison.

La liste des méthodes interdites n'est **pas écrite dans l'axe**. Elle est
déduite du SQL par `scripts/arch/methodes_qui_ecrivent_le_statut.py`, qui
apparie chaque `async fn save_*` du dépôt au fichier qu'elle `include_str!`, et
regarde si ce fichier pose un `status` entre son `SET` et son `WHERE`. Une liste
tenue à la main dérive — ce serait refaire à l'échelle des méthodes l'erreur
qu'on corrige à l'échelle des panneaux.

Deux pièges valaient d'être écrits dans le script, tous deux rencontrés :

- les fichiers `*_keep_status.sql` **expliquent en commentaire** qu'ils ne
  posent pas de statut, contrairement à leur jumeau — la chaîne `status = '…'`
  y figure donc. Un `grep` naïf accuse les deux ; c'est arrivé, et j'ai cru un
  instant que `save_visibility` était fautive ;
- un `WHERE status = …` filtre, il n'écrit pas.

Falsifié : ajouter un `status =` dans `update_structure_keep_status.sql` fait
apparaître `save_structure_and_prune_groups` dans la liste.

## L'e2e : un test paramétré, dans le fichier des choses transverses

Le plan prévoyait trois tests, un par panneau corrigé. C'était reproduire en
petit l'erreur que la carte dénonce. Le test vit donc dans
`test_competition_admin_settings.py` — le fichier dont l'en-tête dit qu'il porte
« les trois choses transverses qu'aucun fichier de panneau ne pouvait porter
seul » — et il est paramétré sur `PANNEAUX`, la liste des cinq. Un sixième
panneau hérite du garde-fou sans que personne y pense.

**Deux défauts du test, mesurés avant qu'il ne prouve quoi que ce soit.**

Il réutilisait `_corps_analysable`, l'aide déjà présente dans le fichier. Son
contrat est de passer les extracteurs d'axum — c'est ce qu'il fallait pour
tester des `403`. Le domaine, lui, n'en veut pas : `{"tiers": []}` est ignoré
par le use case, et un `logo_url` vide est refusé par le value object avant même
d'atteindre le use case. Les deux panneaux répondaient `200` **sans rien
écrire**, donc sans jamais toucher au statut : le test était vert alors que le
défaut était en place. Le fichier avertissait pourtant — « un 415 signerait un
corps non analysé, donc un test creux ». Le même piège, un cran plus loin.

D'où le **témoin** : chaque cas relit en base la valeur qu'il vient d'écrire, et
échoue si elle n'a pas changé. Les valeurs sont des **bascules** calculées
depuis l'état courant, jamais des constantes — une constante déjà en place
passerait sans écriture au second passage.

Second défaut : les cinq cas partagent la saison de la fixture. Le premier
panneau fautif empoisonnait les suivants, qui échouaient sur leur garde
d'entrée. Le test accusait quatre panneaux pour trois défauts, dont deux
innocents. Chaque cas remet donc le statut à `ready` avant de partir.

Falsifié : défaut rétabli dans les trois use cases, les trois panneaux fautifs
rougissent et les deux sains restent verts.

## Ce que la carte ne fait pas

**Aucune reprise de données** : personne n'a subi le défaut.

**Aucun changement du magicien.** `save_rules` reste ce qu'il est, à sa place.

## Tests

| Test | Ce qu'il prouve |
|---|---|
| `le_chemin_qui_reecrit_le_statut_n_est_jamais_emprunte` ×3 | le use case appelle la bonne méthode, unitaire |
| `test_aucun_panneau_ne_fait_regresser_la_saison[×5]` | le comportement des cinq panneaux, e2e |
| Axe 16 de `check-arch` | l'interdiction, pour les panneaux à venir |

Les trois faux de `settings/` **refusent** `save_rules` par `unreachable!` au
lieu de la journaliser : chaque test de ces use cases garde donc l'invariant,
sans qu'aucun n'ait à y penser. Le test nommé existe pour que l'intention porte
un nom.

## Checklist

- [x] `update_rules_keep_status.sql`
- [x] `save_rules_keep_status` sur le port et le dépôt
- [x] Les trois use cases de `settings/` l'appellent
- [x] Les trois gardes unitaires, falsifiées
- [x] L'e2e paramétré sur les cinq panneaux, falsifié
- [x] L'axe 16 et son script de dérivation, falsifiés tous les deux
- [x] `make lint && make test && make check-arch`
- [x] `make e2e` — 341 passés, 0 échec
