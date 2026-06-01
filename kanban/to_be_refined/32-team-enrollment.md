# BC `teams` — Inscription dans une compétition → "Inscrite / Prête à jouer"

**Priorité : haute**
**Dépend de :** `31-team-created-listener.md`, carte BC `competitions` (règles d'accession — à créer)
**Contexte :** `teams` (consommateur) ← BC `competitions` (émetteur)

## Objectif

Faire transiter l'équipe de `PendingEnrollment` vers `Enrolled / ReadyToPlay` quand le BC `competitions` confirme son inscription dans une compétition.

---

## Ce qui est défini

- L'émetteur est le **BC `competitions`**
- Les règles d'accession varient selon la compétition :
  - **Accès libre** : inscription confirmée automatiquement si l'équipe est éligible
  - **Accès validé** : demande soumise par le coach, approuvée par un admin
- Dans les deux cas, `competitions` publie un app event `TeamEnrolled` quand l'inscription est effective

---

## Ce qui reste à définir (bloque cette carte)

### Côté BC `competitions` — nécessite une carte dédiée dans `to_be_refined`

- Modèle des **règles d'accession** sur l'agrégat `Competition`
- Flux "accès validé" : UI de demande, notification admin, action d'approbation/refus
- Payload exact de `TeamEnrolled`
- Existe-t-il un `TeamEnrollmentRejected` ? Si oui, impact sur le BC `teams`

### Côté BC `teams`

- Une équipe peut-elle être inscrite dans **plusieurs compétitions** simultanément ?
  - Si oui : l'agrégat porte plusieurs participations, pas un seul `game_phase`
  - Si non : la structure actuelle (carte 28) est suffisante

---

## Checklist (à compléter après raffinage)

- [ ] Payload `TeamEnrolled` figé côté BC `competitions`
- [ ] Décision : inscriptions multiples simultanées ?
- [ ] `team_enrolled_listener::init()` dans `teams`
- [ ] `Team::enroll()` avec garde sur le statut courant
- [ ] `competition_id` + `season_id` sur l'agrégat si pertinent
- [ ] Test unitaire de la transition
- [ ] Test d'intégration : event → base de données
