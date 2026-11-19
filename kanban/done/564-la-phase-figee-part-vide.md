# La phase 1 figée part vide, et doit passer

**Priorité : haute — des coachs ne peuvent pas commencer leur rapport de match**
**Épic :** aucune — une correction, livrable d'un bloc
**Dépend de :** rien
**Fichiers :**
`src/app/match_report/io/web/match_selection_controller.rs`,
`src/app/match_report/io/web/tests/test_update_match_selection_form.rs`,
`tests/e2e/test_competition_hors_calendrier.py`

## L'objectif

Un coach non administrateur, dans une compétition qui interdit les matchs hors
calendrier, ouvre le brouillon de son rapport, clique sur « Commencer → » et
arrive en phase 2.

## Ce qui l'a fait naître

En production, certains coachs obtiennent en cliquant sur « Commencer → » :

```
failed to deserialize form body: missing field `competition_id`
```

**Qui :** les non-administrateurs d'une compétition dont la saison refuse le
hors-calendrier (carte 550) — ceux pour qui `selection_figee` rend `Some(..)`.

**Pourquoi :** en lecture seule, `match-selection.html` remplace les deux
widgets par les noms en clair. Le formulaire n'a alors **plus aucun champ**, et
le navigateur envoie un corps vide. Or `update_match_selection` extrayait
`Form<CreateMatchReportForm>`, qui exige les cinq champs de la création :
l'extraction échouait avant même le handler.

Le plus ironique : dans ce cas, `equipes_retenues` écarte de toute façon les
valeurs reçues et reprend celles du brouillon. On exigeait des champs pour les
jeter. Et ce POST est le seul chemin de `Draft` vers `PreMatch`.

## Pourquoi les tests ne l'ont pas vu

`test_un_post_trafique_ne_change_pas_les_equipes` (carte 550) envoie un POST
**forgé avec un corps complet**. Il éprouvait la garde, jamais le formulaire
que le gabarit produit réellement. Personne ne cliquait sur le bouton.

## Le changement

- `update_match_selection` reçoit son propre formulaire,
  `UpdateMatchSelectionForm`, qui ne porte que `home_team_id` et
  `away_team_id`, tous deux facultatifs. La compétition, la saison et la
  journée ne changent pas après la création : elles ne sont pas lues.
- `equipes_retenues` rend un `Option` : les équipes du brouillon quand la
  sélection est figée, celles du formulaire sinon — et `None`, donc 400, si
  une sélection modifiable arrive sans ses deux équipes.
- `CreateMatchReportForm` ne change pas : la création a besoin des cinq champs.

## Tests

- Unitaire : le formulaire accepte un corps vide, et lit le corps complet que
  le widget envoie.
- E2E : `test_un_membre_simple_commence_son_rapport_depuis_la_phase_figee` —
  **le vrai bouton**, cliqué par un membre simple sur un brouillon neuf ; il
  doit aboutir en `step2`, équipes inchangées.

## Terminé quand

Un non-administrateur d'une compétition sans hors-calendrier ouvre la phase 1
de son rapport, clique sur « Commencer → », et arrive en phase 2.
