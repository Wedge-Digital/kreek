# L'encart du coach connecté

**Priorité : haute — le second chemin de réponse, pour quand l'e-mail se perd**
**Épic :** E16 — Sondage de présence
**Dépend de :** 529, 530, et 516 pour `record_answer`
**Fichiers :** `src/app/competitions/io/web/widgets/presence_call_widget.rs`,
`.../templates/widgets/presence-call.html`,
`.../templates/competition-detail.html`,
`src/app/competitions/use_cases/presences/presence_call_service.rs`,
`assets/static/css/pages/presence-call.css`, `src/web/css_bundle.rs`,
`routes.rs`

## L'objectif

Sur la page de détail d'une compétition, un coach voit qu'on attend sa réponse et
répond sur place — sans e-mail, sans quitter l'onglet qu'il lisait.

## Ce que le serveur choisit

```
rien du tout                                   <- aucune campagne ne concerne ce coach
une carte par campagne ouverte, échéance croissante
    une ligne par équipe du coach
        selon sa réponse : deux boutons, ou un état et un bouton de bascule
```

Les cinq états de la maquette n'en sont pas cinq : « deux équipes » est le cas
général dont « présent » et « absent » sont le rendu à N=1.

**Une carte par campagne, pas une seule.** R2 garantit une campagne par
*journée*, pas par compétition : deux journées peuvent être sondées en même
temps. Cacher une question parce qu'une autre existe ferait manquer une échéance.

## Le service d'hydratation

```rust
// arch:no-instrument — service d'hydratation : assemble une vue, sans intention métier
pub async fn hydrater(season_id, coach_id, survey_repo, team_port, maintenant) -> Vec<CampagneOuverte>
```

Il croise les campagnes ouvertes, les équipes engagées du coach et les réponses.
**`TeamInfoDto` ne sort pas d'ici** — ni handler, ni gabarit ne le voient.

`confirmes` vient de **`compte_presents()`**, jamais d'un `filter().count()` : la
vue ne voit que les équipes du coach, elle afficherait « 1 équipe a confirmé » là
où il y en a neuf. C'est la carte 495 en plus visible.

## Les deux handlers, dans un fichier

`get_presence_call` et `post_presence_call_answer`. L'action rend **le même
fragment** que le GET ; les séparer ferait deux fichiers dont l'un compterait
vingt lignes.

Tous deux appellent `saison_de_la_competition` (carte 530) — **et rien d'autre** :
n'importe quel coach voit l'encart, qui ne montre que ses propres équipes.

Le POST construit `RecordAnswerCommand` avec **`Repondant::Coach(id du
connecté)`** (R28). L'identité vient de la session, jamais du corps.

| Issue | Réponse |
|---|---|
| succès | l'encart réaffiché, `200` |
| `SurveyClosedForCoach`, `RoundFrozenByReport` | une **carte explicative**, `200`, sans boutons |
| `TeamNotOwnedByCoach`, `TeamNotInSurvey` | `403` — htmx ne remplace rien |

**La carte explicative n'est pas une politesse.** Sans elle, un clic arrivé après
la clôture réafficherait un fragment vide, l'encart disparaîtrait sous le
curseur, et le coach lirait ce vide comme un succès.

**Un `403` qui ne rend rien est correct** : ces deux refus ne viennent que d'un
`team_id` forgé, il n'y a pas d'utilisateur à renseigner. Une carte « cette
équipe n'est pas la vôtre » confirmerait à qui essaie que l'équipe existe.

## R30 — le désistement après tirage

Si `record_answer` rend `EnregistreeRencontreARefaire`, l'encart le dit — « ta
rencontre était déjà tirée ; l'organisateur en sera informé » — et s'arrête là.

**Aucun nouvel adversaire annoncé** : la réparation est une proposition que
l'organisateur valide (carte 518). En annoncer un produirait deux coachs qui se
croient appariés sur un match qui n'existe pas. **Et pas de silence non plus** :
le coach vient de défaire un match que son adversaire avait noté.

## L'insertion, et le détail qui compte

```html
<div hx-get="{{ app_routes.competitions.presence_call(space_id, competition_id, season_id) }}"
     hx-trigger="load"
     hx-swap="outerHTML"></div>
```

Juste avant `<div class="tabs">`, dans `detail-right`.
**`outerHTML` et non `innerHTML`** : le conteneur disparaît avec sa réponse, donc
`campagnes` vide ne laisse aucune trace — pas de `<div>` de zéro hauteur, pas de
marge orpheline, et un espacement de page identique selon qu'un sondage est
ouvert ou non.

`CompetitionDetailTemplate` **ne gagne aucun champ** : la fonction reste
retirable en supprimant ce seul conteneur.

## Le défaut que seul le rendu réel a montré

Le premier jet composait le titre ainsi :

```rust
titre: format!("Seras-tu là pour la {} ?", c.round_name.to_lowercase()),
```

Le `to_lowercase()` était là pour que « Journée 3 » se lise après l'article. Sur
la base de démonstration, dont la journée s'appelle **« J1 »**, ça donnait
**« Seras-tu là pour la j1 ? »**.

C'est exactement la famille « le gabarit n'invente aucune valeur », vue d'un cran
plus haut : ce n'est pas une valeur inventée, c'est une valeur du domaine
**abîmée par la vue**. Le nom de la journée appartient à l'organisateur qui l'a
saisi, et aucune couche d'affichage n'a à le retoucher.

Les cinq tests unitaires ne le voyaient pas — ils portent sur le service, pas sur
le VM — et rien dans la compilation ne le signalait. **Il a fallu lancer une
campagne d'essai et lire le HTML rendu.**

Corrigé en retirant l'article des trois phrases, seule formulation qui reste juste
quel que soit le nom saisi :

| État | Avant | Après |
|---|---|---|
| attendue | « Seras-tu là pour la j1 ? » | « Seras-tu là pour J1 ? » |
| présente | « Tu es attendu à la J1 » | « Tu es attendu pour J1 » |
| absente | « Tu ne joues pas la J1 » | « Tu ne joues pas J1 » |

## La vérification au serveur, et ce qu'elle a couvert

Une campagne lancée sur la base e2e, l'encart lu au `curl`, une réponse postée, le
fragment relu — puis la campagne supprimée et l'absence de réponse orpheline
vérifiée.

Ce que ça a montré et que rien d'autre ne montrait : le conteneur est bien dans la
page de détail · la route rend `200` · **une saison sans campagne ouverte rend un
corps vide**, donc le `outerHTML` efface le conteneur · le POST enregistre et
réaffiche l'état « présente » avec sa pastille, le nom de l'équipe et sa date ·
les `hx-vals` portent les bons identifiants.

## Deux écarts à la carte

**Le service prend `match_day_repo`**, que la signature de la carte omettait.
`CampagneOuverte` porte le nom et les dates de la journée, et `PresenceSurvey` ne
connaît que son `round_id` — il fallait bien les lire quelque part.
`find_by_season` suffit : une requête pour toutes les journées, pas une par
campagne.

**La racine du widget porte `.presence-call`, les cartes portent `.pc-card`.** La
maquette nommait la carte `.presence-call` et n'en prévoyait qu'une. R2
garantissant une campagne par **journée**, il peut y en avoir plusieurs ; et l'axe
15 veut que le nom du fichier soit le sélecteur de portée. La racine devient donc
le conteneur, ce qui donne au passage une cible stable au `hx-swap="outerHTML"`
des boutons.

## Ce que l'axe 18 a fait dès sa première carte

Il a refusé `presence_call_widget.rs`, absent de son périmètre. Ce n'est pas un
faux positif : il a forcé à **déclarer** de quel côté de la feuille le nouveau
fichier tombe, au lieu de laisser la réponse se décider toute seule par l'endroit
où on l'avait rangé. Le périmètre a été étendu, et le motif écrit dans le script.

## Checklist

- [x] Deux routes dans `routes.rs`, hors `/admin`
- [x] Le service d'hydratation, `arch:no-instrument` avec son motif
- [x] Les deux handlers, `saison_de_la_competition` sur les deux
- [x] Les VM : `CampagneVm`, `EquipeLigneVm`, `EtatEquipeVm` (un enum, pas trois
      booléens)
- [x] Les gabarits : les deux mises en forme, la carte de refus, celle de R30,
      `hx-disinherit="*"` sur les trois racines
- [x] Les boutons factorisés en un seul partiel — l'URL, les en-têtes htmx et la
      cible du swap à un seul endroit
- [x] Le conteneur dans `competition-detail.html`, en `outerHTML`
- [x] La feuille, inscrite dans `FEUILLES_APP`, portée `.presence-call`
- [x] Le vert des boutons assombri en `#4A7364` — **le token n'est pas touché**
- [x] Cinq tests de service, dont celui qui tient `confirmes`
- [x] Rendu vérifié au serveur, aller et retour, données d'essai nettoyées
- [x] `make lint`, `make check-arch`, `make test`
