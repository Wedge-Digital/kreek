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

- [ ] Fixture : joueur customisable + identité simple
- [ ] Scénario 1 — autorisation
- [ ] Scénario 2 — compétence persistée et marquée
- [ ] Scénario 3 — direction des seuils de dé
- [ ] Scénario 4 — borne
- [ ] Scénario 5 — doublon
- [ ] Scénario 6 — TV déplacée par le prix, **pas** par la compétence
- [ ] Scénario 7 — annulation
- [ ] Scénario 8 — persistance du panier
- [ ] Scénario 9 — prix plancher
- [ ] Scénario 10 — péremption
- [ ] Scénario 11 — cloisonnement des espaces (carte 315)
- [ ] Entrée ajoutée à `tests/impact-map.toml` (skill `test-impact`)
