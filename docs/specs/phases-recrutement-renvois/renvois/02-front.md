# Renvois — Phase 2 : architecture front

**Entrée** : maquette validée `assets/rawpages/html/app-team-dismissals.html`
**Page** : `/app/{space_id}/teams/{team_id}/dismissals`

## Principe

Même modèle que le recrutement : panier serveur, panier persisté, un POST par
mutation qui renvoie un fragment, un événement DOM qui resynchronise l'autre widget.
Se reporter à `recrutement/02-front.md` pour le détail du pattern — ce document ne
consigne que **ce qui diffère**.

Le JavaScript de la maquette disparaît de la même façon ; il ne reste que le repli du
panier sous 768px.

## Trois différences de fond

**Aucune trésorerie à surveiller.** Un renvoi ne rembourse rien, donc rien n'entre ni
ne sort. Aucun bouton n'est jamais désactivé pour cause d'argent, et le panier
n'affiche pas de montant — il affiche l'**effectif après renvois**.

**Aucun quota ne s'applique.** Retirer un joueur ne peut violer ni `max_quantity`, ni
une limite croisée, ni le plafond de 16. Toutes les gardes de composition du
recrutement sont ici sans objet.

**Une seule garde, mais qui mord fort** : le plancher des 11 joueurs éligibles.

## Le plancher des 11 éligibles

On ne peut pas descendre sous **11 joueurs éligibles au prochain match**.

Un joueur absent ne comptant pas parmi les éligibles, le renvoyer n'entame pas le
plancher : **il reste toujours renvoyable**. La garde ne porte donc que sur les
joueurs disponibles.

Conséquence mesurée sur la maquette : une équipe de 14 joueurs dont 2 blessés a 12
éligibles, donc un seul renvoi de joueur valide possible — après quoi les 11 restants
passent tous en « Minimum 11 » et seuls les blessés demeurent renvoyables.

**L'alerte journaliers change de nature.** Elle ne peut plus signaler une conséquence
des renvois, puisque le plancher l'interdit : elle informe désormais d'un déficit
**déjà causé par les blessures**, que le coach subit et ne peut qu'aggraver en
renvoyant des absents.

## Widgets

| Widget | BC | Endpoint | Trigger | Émet | Mode |
|---|---|---|---|---|---|
| `dismissals_roster` | teams | `GET /app/{space_id}/team/widgets/dismissals-roster` | `load, basketChanged from:body` | `basketChanged` (via ses POST) | lecture + mutation |
| `dismissals_cart` | teams | `GET /app/{space_id}/team/widgets/dismissals-cart` | `load, basketChanged from:body` | `basketChanged` (via ses POST) | lecture + annulation + validation |

### Contenu de `dismissals_roster`

- En-tête de contexte : roster, trésorerie, effectif, **éligibles au prochain match**
- Avertissement de phase : un renvoi validé est définitif et ne rembourse rien
- Tableau de l'effectif : numéro, nom, poste, SPP, valeur, statut de participation, bouton
- Tableau du staff : fonction, effectif `possédé (−en attente)`, valeur unitaire, bouton

Comme au recrutement, le tableau se rafraîchit **entier** : marquer un joueur peut
faire basculer tous les autres en « Minimum 11 » d'un seul coup.

### Contenu de `dismissals_cart`

- Lignes du panier avec un bouton d'annulation
- Effectif après renvois
- Alerte journaliers quand les éligibles passent sous 11
- Bouton de validation de phase
- État vide : « Aucun renvoi en attente »

## Trois états par ligne de joueur

C'est la particularité de cette page, absente du recrutement :

| État | Bouton | Couleur |
|---|---|---|
| Renvoyable | « Renvoyer » | neutre, rouge au survol |
| Marqué pour renvoi | « Annuler » | bleu — **seule action réversible de l'écran** |
| Bloqué par le plancher | « Minimum 11 », désactivé | neutre atténué |

La ligne marquée reste **lisible**, barrée mais jamais estompée à l'opacité : c'est la
trace de ce que le coach vient de décider.

## Actions

| Verbe | Route | Corps | Réponse |
|---|---|---|---|
| `POST` | `…/dismissals/players/mark` | `player_id`, `version` | fragment effectif + `HX-Trigger: basketChanged` |
| `POST` | `…/dismissals/players/unmark` | `player_id`, `version` | fragment effectif + `HX-Trigger: basketChanged` |
| `POST` | `…/dismissals/staff/mark` | `staff_uid`, `version` | fragment effectif + `HX-Trigger: basketChanged` |
| `POST` | `…/dismissals/staff/unmark` | `line_id`, `version` | fragment panier + `HX-Trigger: basketChanged` |
| `POST` | `…/validate-dismissals-phase` | `version` | `HX-Refresh: true` — **route existante**, rôle élargi |

**`mark` / `unmark`, et non `add` / `remove`.** Sur une page de renvois, une route
nommée `players/add` se lit « ajouter un joueur à l'équipe » — exactement l'inverse de
ce qu'elle fait. La symétrie avec le recrutement ne vaut pas ce risque de contresens.

## Ports nécessaires

| Cible | Données |
|---|---|
| `players` | effectif détaillé : nom, poste, SPP, valeur, statut de participation |
| `references` | prix du staff, `allowed_staff`, prix de base de relance |

Le besoin est **plus riche qu'au recrutement**, qui ne demandait que des compteurs par
ligne de roster. Ici il faut la liste nominative.

Cette liste appartient à `players` — pourquoi un port et non un widget de `players` ?
Parce que chaque ligne porte un bouton d'action de `teams`, et qu'un widget de
`players` ne peut pas rendre une action d'un autre BC. Le port reste donc le seul
chemin, comme au recrutement.

## Staff

| Élément | Renvoyable |
|---|---|
| Relance | ✅ — **à ouvrir**, le domaine la refuse aujourd'hui |
| Assistant entraîneur | ✅ |
| Pom-pom girl | ✅ |
| Apothicaire | ✅ |
| Facteur fans | ❌ |

Seule garde : ne pas renvoyer plus que ce que l'on possède, en tenant compte des
lignes déjà en attente dans le panier.

Quand le roster n'a pas droit à l'apothicaire, la ligne **n'apparaît pas** — non
parce que le renvoi serait interdit, mais parce que l'équipe ne peut pas en posséder.

## Règles métier identifiées à cette étape

- L'en-tête affiche **effectif** et **éligibles au prochain match** comme deux nombres
  distincts : c'est le second qui gouverne le plancher, et confondre les deux rendrait
  le blocage incompréhensible.
- La valeur d'un joueur reste affichée alors qu'elle ne sera pas remboursée. C'est
  volontaire : elle mesure ce que le coach s'apprête à perdre.
- Aucune confirmation par boîte de dialogue. Elle protégeait d'un geste irréversible ;
  le panier rend chaque ligne annulable jusqu'à la validation, la confirmation n'a
  plus d'objet.

## Conséquence à porter en phase 3

`refund_kpo` vaudra **toujours zéro**. Le bras `apply(StaffDismissed)` crédite
aujourd'hui la trésorerie de ce montant (`teams/domain/team.rs:477`) : le paramètre
devient un vestige, à retirer ou à assumer explicitement.

## Points ouverts pour la phase 3

- Panier partagé avec le recrutement — table unique discriminée par phase — ou
  table dédiée ? La question se pose identiquement pour les deux pages.
- Un joueur marqué puis devenu indisponible entre-temps (dépublication de rapport) :
  le plancher se recalcule, la ligne peut devenir invalide. Le refus en bloc de la
  décision D5 s'applique, mais il faut décider ce que le fragment d'erreur montre.
