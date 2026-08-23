# Page hôte + onglet Membres — Architecture front

**Maquette :** `assets/rawpages/html/app-space-admin.html`, bannière, barre
d'onglets, et bloc `#tab-members`.

## La page hôte ne porte rien

Bannière, badge de visibilité, barre d'onglets. Aucune logique, aucun calcul de
VM, aucun JS d'orchestration — c'est le patron « page d'assemblage à widgets »
du `CLAUDE.md`.

Le badge de visibilité de la bannière est rendu par la page, pas par un widget :
c'est une donnée de l'espace, disponible au moment où la page est rendue, et
elle ne change qu'à l'onglet Paramètres.

## Les widgets

| Widget | BC | Endpoint | Trigger | Émet | Mode |
|---|---|---|---|---|---|
| `space-admin-stats` | spaces | `GET /app/{space_id}/admin/widgets/stats` | `load`, `memberAdded from:body`, `memberRemoved from:body`, `memberRoleChanged from:body` | — | lecture |
| `space-admin-members` | spaces | `GET /app/{space_id}/admin/widgets/members` | `load`, `memberAdded from:body` | `memberRemoved`, `memberRoleChanged` | mutation |

## Événements

```
memberAdded       { coach_id, name }   émis par : onglet Ajout direct
                                       écouté par : stats, liste des membres

memberRemoved     { coach_id }         émis par : liste des membres
                                       écouté par : stats, candidats de l'Ajout direct

memberRoleChanged { coach_id, role }   émis par : liste des membres
                                       écouté par : stats
```

`memberRemoved` a un second consommateur qui n'existe pas encore : la liste des
candidats de l'Ajout direct affiche un badge « Déjà membre » sur les coachs
présents dans l'espace, et doit le lever quand l'un d'eux en sort. Le contrat
est posé maintenant pour que l'onglet Ajout direct n'ait rien à renégocier.

## Actions

```
POST /app/{space_id}/admin/members/{coach_id}/role
     → remplace la ligne          HX-Trigger: memberRoleChanged

POST /app/{space_id}/admin/members/{coach_id}/remove
     → retire la ligne            HX-Trigger: memberRemoved

POST /app/{space_id}/admin/members/{coach_id}/reset-password
     → aucun swap                 HX-Trigger: showToast
```

Le changement de rôle **remplace la ligne** plutôt que de ne rien renvoyer : le
serveur est seul juge de ce que la ligne doit afficher ensuite — le `select` du
dernier administrateur doit se figer dès qu'il le devient. Renvoyer la ligne
rend cet état sans que le client ait à le déduire.

## Front contre back

**Au front, la recherche de membre.** Filtre Alpine sur les lignes déjà rendues.
La liste des membres d'un espace tient dans un écran ; un aller-retour par
frappe n'achèterait rien.

C'est le seul écart avec `coach-search`, qui interroge le serveur — mais lui
cherche dans l'annuaire de la plateforme, dont la taille est inconnue et croît
sans rapport avec l'espace. Les deux choix sont justes pour leur liste.

**Au back, tout le reste.** Changement de rôle, retrait, réinitialisation :
aucun des trois n'a d'état intermédiaire côté client, et les trois portent une
règle que seul le serveur peut trancher.

## Le chargement des onglets

Chaque onglet est un widget chargé **à sa première activation**, pas au
chargement de la page. Une fois chargé, il reste dans le DOM ; rebasculer
dessus ne redéclenche rien.

Conséquence directe, et elle est à traiter dès la conception : **un onglet
chargé en différé pousse le contenu en arrivant**, exactement le défaut des
cartes 343 et 361. La zone d'onglet doit réserver sa hauteur.

Le critère de plancher de la carte 361 s'applique tel quel : la réservation doit
venir d'une règle, pas d'une estimation. Ici la règle existe — **un espace a
toujours au moins un administrateur**, donc la liste a toujours au moins une
ligne, et le panneau a un plancher exact. Réserver au-delà créerait un blanc
permanent sur les petits espaces.

## Le compteur d'invitations en attente

La barre de statistiques compte les membres, les administrateurs et les
invitations en attente. Le troisième dépend d'un onglet qui n'existe pas encore.

Le widget est livré avec ce compteur **à zéro**, et l'onglet Invitations
n'ajoutera qu'une requête. L'alternative — livrer deux compteurs et rouvrir la
carte plus tard — découpe un widget en deux moitiés dont la seconde n'a pas de
valeur propre.

## Widgets existants — ce qui ne se réutilise pas

`space-members-widget` porte le nom de ce qu'on construit et ne fait pas la même
chose : c'est un **sélecteur** de coachs pour formulaires, avec un mode simple
et un mode multiple, pas une liste d'administration. Aucune réutilisation.

`coach-search` cherche **parmi les membres de l'espace**
(`list_members_for_space`), ce qui est l'inverse du besoin de l'Ajout direct.
Sans objet pour l'onglet Membres.

## Conventions à respecter

- Racine de chaque widget en `hx-disinherit="*"`.
- Aucun `<link rel="stylesheet">` : les feuilles s'inscrivent dans
  `src/web/css_bundle.rs`, dans l'ordre imposé, et sont nommées d'après la
  racine du widget qui les porte.
- Aucun `style="…"` : la maquette en contient, ils ne se transcrivent pas.
- Le `select` de rôle est un **`<kreek-select>`**, les `<select>` natifs étant
  interdits hors maquette.
- Scripts scopés par `document.currentScript.previousElementSibling`, jamais par
  un `id` global.

## Règles métier de cet onglet

1. **Un espace a toujours au moins un administrateur.** Le dernier ne peut être
   ni rétrogradé, ni retiré, par personne — lui compris. C'est un invariant de
   l'agrégat `Space`, pas une garde d'interface : le front grise, le domaine
   refuse.
2. **On ne modifie pas son propre rôle.** Le `select` de sa propre ligne est
   désactivé.
3. **On ne se retire pas soi-même.** Pas de bouton de retrait sur sa propre
   ligne.
4. **Retirer un coach est autorisé même s'il a une équipe engagée.**
5. **La réinitialisation envoie un email**, y compris à soi-même.
6. **Seul un administrateur accède à la page** — `SpacePermissions::is_admin()`,
   sur la page comme sur chacun des endpoints de widget et d'action. Un widget
   n'hérite d'aucune protection de sa page hôte.

## Questions ouvertes pour la phase 3

- Le retrait d'un membre doit-il émettre un app event ? Les autres BCs cachent
  des données de membres — `competitions__user_space_cache` en est une — et
  rien ne les préviendra sans lui.
- La promotion doit reprendre `UserPromotedToSpaceAdmin`, défini et jamais
  émis. Reste à décider s'il faut son symétrique pour la rétrogradation, ou un
  seul événement portant le rôle cible.
