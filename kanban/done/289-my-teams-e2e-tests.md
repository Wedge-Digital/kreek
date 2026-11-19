# Tests E2E — page "Mes équipes"

**Priorité : haute**
**Dépend de :** `287-teams-my-teams-widget.md`, `288-team-creation-my-teams-page.md`
**Contexte :** `team_creation` + `teams` — tests Playwright

## Objectif

Couvrir en E2E le comportement réel de la page restructurée : c'est
précisément le genre de bug (statut affiché incohérent avec le statut
domaine réel) qu'aucun test unitaire n'aurait détecté seul.

**Spec de référence :** `docs/specs/my-teams/mes-equipes/07-integration.md`.

---

## Scénarios

1. Coach avec : 2 brouillons (un avec roster choisi, un sans), 2 équipes
   actives avec `game_phase` différents, 1 équipe refusée, 1 équipe
   renvoyée → les 3 sections affichent le bon nombre de cartes et les bons
   libellés (dont "Phase d'amélioration"/"Phase de recrutement" etc., pas
   des libellés inventés).
2. Clic sur "Continuer →" d'un brouillon → navigation vers la page de build
   de ce brouillon.
3. Clic sur une carte active ou archivée → navigation vers le détail de
   l'équipe correspondante.
4. Coach sans aucune équipe dans le space → les 3 sections sont absentes du
   DOM (pas de titre "0").
5. Équipe fraîchement soumise (juste après `mark_submitted`) : disparaît de
   "En cours de création" — vérifie que le bug initial (statut "Inscrite"
   affiché avant tout traitement admin) est bien corrigé.

---

## Checklist

- [x] Fixture : coach avec équipes dans les 4 `ParticipationStatus`
- [x] Scénario 1 — répartition et libellés des 3 sections
- [x] Scénario 2 — navigation brouillon → build
- [x] Scénario 3 — navigation active/archivée → détail
- [x] Scénario 4 — état vide, aucune section affichée
- [x] Scénario 5 — non-régression du bug initial (statut affiché = statut réel)
- [x] Carte ajoutée à la carte d'impact tests↔bounded-contexts (skill `test-impact`)

---

## Notes d'implémentation

Les cinq scénarios décrivaient correctement le comportement réel : les quatre
corrections nécessaires pour passer au vert sont toutes dans le test, aucune
dans le code applicatif.

1. **Enrôlement asynchrone.** `dismiss` était appelé pendant que l'équipe
   était encore `PendingEnrollment` — `Team::dismiss()` n'accepte que
   `Enrolled`, d'où un 422 qui ressemblait à un bug applicatif. Helper local
   `_wait_status()` sur `team_proj`, appelé pour les quatre équipes de la
   compétition auto et pour l'équipe rejetée (`PendingEnrollment`, l'autre
   bout de la même course).
2. **`form=` et non `data=`** pour `/app/space/create` : `register_space_submit`
   prend un `Form<…>`, un dict passé à `data=` part en JSON (415).
   `_create_draft` garde `data=`, son handler prenant un `Json<…>`.
3. **Journées de compétition.** La phase 3 ne fait que poster la structure ;
   c'est `sync_and_generate_schedule()` qui alimente
   `competition_match_days`. La fixture sautait l'étape.
4. **`networkidle` remplacé** par l'attente du `<link>` du widget : les logos
   distants gardent le réseau actif, l'attente n'aboutissait jamais. C'était le
   seul usage de `networkidle` de toute la suite.

Suite complète sur base fraîchement reset : `5 failed, 155 passed, 7 skipped`,
les cinq échecs étant préexistants et sans rapport avec cette carte (panier
mobile chevauchant la tabbar, et débit de recrutement erroné).
