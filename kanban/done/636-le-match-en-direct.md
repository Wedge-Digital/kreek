# Le match en direct

**Épic :** E21 — Le mode de saisie et l'après-match en séquence · **Ordre :** 6 · **Dépend de :** 635
**Conception :** `docs/specs/saisie-mobile/E21-cadrage.md` (R5, R9, § 4)
**Maquettes :** `assets/rawpages/html/app-mobile-solo-live.html`, `app-mobile-match-new-action.html`, `app-mobile-solo-follow.html`

## Objectif

Le match se saisit depuis **un écran de journal** : le score, le journal des
deux camps, « ＋ Nouvelle action », « Fin du match → MVP ». « Nouvelle action »
demande l'équipe, puis ouvre l'étape de saisie de ce camp. L'autre coach ouvre
le même écran, sans aucun bouton d'écriture : « Yann saisit tout le match ».

## Conception

- **Route `GET /{id}/live`**, dans `routes_de_suivi()` (634) : bandeau des deux
  équipes et score (`compute_score`), le widget de 635, et selon
  `can_write_report` les boutons ou le bandeau de suivi.
- **« Quelle équipe ? »** : un choix dans la page (domicile, visiteur), qui
  mène à `step3` ou `step4`. Pas de route propre.
- **`step3`/`step4`** (`io/web/actions_step_controller.rs:116-133`) : un seul
  retour, « ← Retour au journal », vers `/live` ; plus de bouton « suite ».
- **Les enchaînements** : la fin de l'avant-match et des coups de pouce mène à
  `/live` au lieu de `step3` (`pre_match_controller.rs:228`,
  `inducements_controller.rs:270`). `edit_match_report` sur un `PreMatch` mène
  à `step2` comme aujourd'hui.
- **La lecture seule** : le refus en page de 634 devient une redirection 303
  vers `/live`.
- **L'indicateur d'étapes** : « 2 · Match » actif sur `/live`, `step3`,
  `step4`. Un fragment commun `mr-steps` remplace les six copies — l'ancien
  découpage (Sélection · Avant-match · Domicile · Visiteur · Après-match) cède
  la place aux cinq étapes de R9 partout où il apparaît.

## Données de production

Aucune. Un compte rendu en cours à la livraison reprend au match en direct ; ses
actions déjà saisies y sont.

## Checklist

- [x] `/live` et son gabarit, sa feuille au bundle ; le choix d'équipe
- [x] `step3`/`step4` : « Retour au journal » seul
- [x] Avant-match et coups de pouce mènent à `/live` ; refus de 634 → redirection
- [x] Fragment `mr-steps` à cinq étapes, inclus par chaque gabarit
- [x] Tests unitaires : le VM de `/live` avec et sans droit d'écriture
- [x] E2E : de l'avant-match au journal, « Nouvelle action » → équipe → saisie → retour ; l'autre coach ne voit aucun bouton ; `test_match_report_step3_4_actions.py` suit la navigation
- [x] `tests/impact-map.toml` et `tests/e2e/visual/urls.py` mis à jour
- [x] `make lint`, `make check-arch`, `make test`, e2e impactés
