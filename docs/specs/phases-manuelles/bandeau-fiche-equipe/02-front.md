# Bandeau de la fiche équipe · Phase 2 : architecture front

**Maquette** : `assets/rawpages/html/app-team-phase-override.html`

## Ce n'est pas un widget

Comme le panneau d'ajustement de trésorerie, et pour la même raison : un
formulaire et une réponse, un seul BC. Le `CLAUDE.md` range ce cas sous « Quand
NE PAS appliquer » le patron d'assemblage.

Le bouton et le panneau sont rendus **par le gabarit de la page**,
`teams-team-detail.html`, dans le bandeau qui existe déjà.

| Élément | Endpoint | Trigger | Émet | Mode |
|---|---|---|---|---|
| bandeau et panneau | `team_detail` (GET, existant) | chargement de la fiche | — | lecture, **plus le panneau si commissaire et équipe prête à jouer** |
| ouverture d'une phase | `POST /app/{space_id}/teams/{team_id}/phases/override` (neuf) | soumission du formulaire | — | mutation |
| fermeture d'une phase | `validate-improvement-phase`, `validate-recruitment-phase`, `validate-dismissals-phase` (existants) | boutons habituels de chaque phase | — | **inchangé côté front** |

**Aucun événement DOM**, ni émis ni écouté : un seul acteur, et la réponse rend
tout ce qui a changé.

**Aucun `hx-disinherit`** : ni `teams-team-detail.html` ni `app-layout.html` ne
posent d'attribut HTMX hérité — la vérification déjà faite pour la trésorerie.

## La fermeture ne demande rien au front

Les trois sorties ramènent déjà à la fiche : `HX-Refresh` pour la validation
des évolutions, `HX-Redirect` vers la fiche pour le recrutement et les renvois
(`validate_phase_actions.rs`). C'est le **domaine** qui décide, selon l'entrée
de phase, si l'équipe enchaîne ou redevient prête à jouer — le front ne voit que
le résultat.

Pour les renvois, l'issue `ErreursCouteuses` mène à l'écran du jet ;
`ValidateDismissalsOutcome::depuis_le_lot` la déduit du dernier événement du
lot. Une sortie manuelle ne produisant jamais `CostlyMistakesPhaseStarted`,
l'issue est `PreteAJouer` sans rien changer au contrôleur.

## La réponse en cas de succès

**`HX-Refresh: true`**, comme la validation des évolutions
(`validate_phase_actions.rs:31`).

Ouvrir une phase change trois choses visibles : le badge de l'en-tête — **hors**
de la zone d'onglets —, le bandeau, et l'onglet Effectif, qui perd « Modifier
l'effectif ». Recharger met tout à jour d'un coup, et garde l'onglet affiché
puisque chaque onglet a son URL. Un rendu de page imposé (`rendre_fiche` avec un
onglet fixé, comme la trésorerie) ramènerait sur l'onglet Effectif un
commissaire qui regardait les matchs.

## La réponse en cas d'erreur

Le panneau reste ouvert, la saisie reste à l'écran, et le message s'affiche dans
son pied. Le mécanisme de la trésorerie, sur le modèle de `submit_error_response`
(`finalize_team.rs:51-62`) :

```
HX-Retarget: #phase-override-error   HX-Reselect: #phase-override-error   HX-Reswap: outerHTML
```

**Le message seul, pas le pied entier** (décision du 2026-10-03, carte 578) : le
pied porte le bouton nommé par Alpine, et la carte 586 a appris qu'un pied
remonté effaçait aussitôt ce que le serveur venait de dire. Le conteneur
`#phase-override-error` existe toujours, vide et masqué, jusqu'au premier
refus.

**`HX-Reselect` n'est pas une précaution** : sans lui, le `hx-select` du
formulaire filtre aussi la réponse d'erreur, n'y trouve pas sa cible, et rien
ne s'affiche — le piège documenté dans `finalize_team.rs:44-50`.

**Statut 200, pas 4xx** : htmx n'échange pas une réponse non-2xx par défaut.

Les refus attendus : l'équipe n'est plus prête à jouer (un rapport de match
s'est ouvert entre-temps), le visiteur n'est pas commissaire.

## Ce qui reste au front

Un `x-data` Alpine sur le bandeau, et rien d'autre :

- l'ouverture et la fermeture du panneau ;
- la phase choisie ;
- le bouton nommé d'après elle — « Ouvrir le recrutement » — et grisé tant
  qu'aucune n'est choisie.

Le motif est un champ libre, facultatif.

**Alpine et non un `<script>` nu** : le bandeau est remplacé à chaque swap de la
page ; un `x-data` se ré-initialise de lui-même (règle 7 des widgets, appliquée
pour la même raison à un composant qui n'en est pas un).

**Le bouton n'apparaît que pour un commissaire, et seulement depuis « prête à
jouer ».** Le serveur revérifie les deux : l'affichage est un confort, pas une
garantie.

## CSS

Les règles du panneau rejoignent `pages/team-page.css`, sous `.team-page`. Pas
de feuille neuve : elle n'aurait pas de racine à elle, et l'axe 14 de
`check-arch` exigerait de l'inscrire au bundle pour un fragment de la même page.

## Responsivité

Desktop-first, breakpoint unique à 768 px. Sous ce seuil : les trois cartes
passent en colonne, les boutons du pied en pleine largeur, « Ouvrir » au-dessus
d'« Annuler ». Le reste de l'écran est déjà tenu par la fiche et
`app-layout.html`.

## Règles métier

Fixées en phase 1 :

1. L'ouverture n'est possible **que depuis « prête à jouer »**.
2. Elle est réservée aux **admins d'espace ou de compétition**.
3. On choisit **l'une des trois phases** — dépense des SPP, recrutement,
   renvois —, avec un **motif facultatif**, enregistré avec le nom de l'admin.
4. L'ouverture est possible **dans tous les cas** : même si aucun joueur n'a de
   SPP, même avec une trésorerie vide.
5. Le **coach ou un admin** ferme la phase, avec le bouton habituel.
6. À la sortie, l'équipe **redevient prête à jouer**, sans enchaîner sur les
   phases suivantes **ni sur les erreurs coûteuses**. La valeur d'équipe et la
   projection sont recalculées.
7. **Le libellé à l'écran ne change pas** : une phase manuelle s'affiche comme
   une phase d'après-match.
8. Une ouverture par erreur **se corrige par la fermeture** ; il n'y a pas
   d'annulation.

### Rien de neuf à cette étape

Question posée le 2026-10-03 : la phase 2 n'ajoute aucune règle métier. Elle
décide d'une composition et d'un mécanisme de réponse, pas de ce qui est permis.
