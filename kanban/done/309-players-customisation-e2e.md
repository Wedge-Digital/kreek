# Tests E2E — mode customisation

**Priorité : haute**
**Dépend de :** `307-players-customisation-widget.md`, `308-players-customisation-endpoints.md`
**Contexte :** `players` — tests Playwright

## Objectif

Couvrir en navigateur ce qu'aucun test unitaire ne voit : la table des
directions de bout en bout, l'asymétrie de la valeur d'équipe, et
l'autorisation resserrée.

**Spec :** `07-integration.md`.

---

## Fixture

Un joueur d'une équipe de l'espace E2E, et **la seconde identité de
`bypass_auth`** (`X-Bypass-Auth-Profile: simple`) pour l'autorisation —
introduite en carte 295, c'est exactement son cas d'usage.

## Scénarios

1. **Membre simple** — le bouton n'existe pas, et le `POST` direct répond 403.
2. **Ajouter une compétence**, valider, recharger → elle figure sur le joueur,
   et le journal la marque `🛠️ Customisation`.
3. **Améliorer l'agilité** → l'affichage passe de `3+` à `2+`. Vérifie la table
   des directions de bout en bout.
4. **Améliorer jusqu'à la borne** → le bouton se grise, et le `POST` forcé est
   refusé avec son motif.
5. **Compétence déjà possédée** → refusée, motif affiché à côté d'elle.
6. **Ajuster le prix** → la valeur du joueur change **et la valeur d'équipe
   suit**. Puis : après une customisation de compétence, la TV **ne bouge
   pas**.
7. **Annuler** → panier disparu, journal revenu, et un rechargement ne rouvre
   pas le mode.
8. **Recharger en cours de saisie** → panier retrouvé intact, mode rouvert.
9. **Prix sous zéro** → refusé.
10. **Panier périmé** — `updated_at` reculé de plus de 24 h → la fiche retombe
    sur le journal, le panier a disparu, le message d'abandon s'affiche.

11. **Espace étranger** — le même joueur, appelé depuis un espace dont
    l'utilisateur est admin mais auquel le joueur n'appartient pas → `404` sur
    la fiche, sur le panneau **et** sur un `POST` de mutation. Jamais `403` :
    rien ne doit confirmer l'existence d'un joueur d'un autre espace.

Le **scénario 6** est celui qui protège la règle la plus contre-intuitive de la
fonctionnalité — la seule qu'un lecteur de bonne foi prendrait pour un bug.

Le **scénario 3** est l'autre pilier : sans lui, une inversion de la table des
directions passerait tous les tests unitaires.

---

## Checklist

- [x] Fixture : joueur customisable + identité simple
- [x] Scénario 1 — autorisation
- [x] Scénario 2 — compétence persistée et marquée
- [x] Scénario 3 — direction des seuils de dé
- [x] Scénario 4 — borne
- [x] Scénario 5 — doublon
- [x] Scénario 6 — TV déplacée par le prix, **pas** par la compétence
- [x] Scénario 7 — annulation
- [x] Scénario 8 — persistance du panier
- [x] Scénario 9 — prix plancher
- [x] Scénario 10 — péremption
- [x] Scénario 11 — cloisonnement des espaces (carte 315)
- [x] Entrée ajoutée à `tests/impact-map.toml` (skill `test-impact`)

## Réalisé

`tests/e2e/test_player_customisation.py` — onze tests, verts en 7 s.

`db_helpers.execute_db` ajouté, nommé et documenté pour ce qu'il est : le
moyen de fabriquer un état qu'aucun parcours utilisateur ne peut atteindre.
Un seul usage, le vieillissement du panier au scénario 10.

### Les scénarios sont surtout en HTTP, et c'est délibéré

Le panneau étant rendu par le serveur, le navigateur n'ajoute rien à la plupart
des assertions — même arbitrage que `competition_lifecycle` et
`match_report_helpers`. Il reste là où le rendu **fait partie de ce qu'on
affirme** : la présence du bouton pour un commissaire.

Le scénario 1 a dû quitter le navigateur : `set_extra_http_headers` pose
`X-Bypass-Auth-Profile` sur **toutes** les requêtes de la page, polices Google
comprises, dont le préflight CORS échoue alors. L'échec ne disait rien de la
fonctionnalité.

### Deux découvertes en écrivant les tests

**Un test qui serait passé pour rien.** Le scénario 8 cherchait
`widgets/customisation` n'importe où dans la fiche — or le bouton « Customiser »
porte cette URL en permanence dans son `hx-get`. Il aurait été vert sur une page
où le mode ne s'était pas rouvert. `_slot_droit()` cible désormais
`#pd-right-panel` précisément.

**Une régression réelle**, dans un test existant :
`test_player_detail::test_customise_button_present_but_disabled` affirmait un
bouton désactivé, ce que la carte 307 a changé. Renommé et réécrit — le bouton
est actif pour un commissaire, et **absent** pour les autres.

### Le scénario 5 ne lit pas les compétences de base

Elles sont stockées en JSON dans la projection, sous une forme dont rien ne
garantit que ce soit l'identifiant attendu par l'endpoint. Le test acquiert
d'abord la compétence **par customisation**, puis la redemande : identifiant
sûr, et la règle est exercée sur une compétence acquise — le cas que la phase 1
visait.
