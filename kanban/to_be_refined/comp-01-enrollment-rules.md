# BC `competitions` — Règles d'accession et inscription d'équipe

**Priorité : haute**
**Dépend de :** `30-team-created-app-event.md`
**Contexte :** `competitions` (émetteur de `TeamEnrolled`) → `teams` (consommateur)

## Objectif

Modéliser dans le BC `competitions` les règles d'accession à une compétition, le flux d'inscription d'équipe (libre ou validé), et la publication de l'app event `TeamEnrolled`.

---

## Ce qui est défini

Une compétition a une **règle d'accession** qui détermine le mode d'inscription :
- **Accès libre** : toute équipe éligible s'inscrit directement → `TeamEnrolled` émis immédiatement
- **Accès validé** : le coach soumet une demande → un admin approuve ou refuse → `TeamEnrolled` émis seulement après approbation

---

## Ce qui reste à définir

### Règles d'éligibilité

- Quelles conditions doit remplir une équipe pour pouvoir s'inscrire ?
  - Valeur d'équipe (TV min/max) ?
  - Roster autorisé ?
  - Statut (doit être `PendingEnrollment` dans BC `teams`) ?
- Qui vérifie l'éligibilité : le BC `competitions` ou le coach est libre de soumettre n'importe quelle équipe ?

### Flux "accès validé"

- Comment le coach initie-t-il la demande ? (bouton sur la page compétition ? sur la fiche équipe ?)
- Comment l'admin est-il notifié ? (liste des demandes en attente, notification ?)
- L'admin peut-il refuser avec un motif ? Ce motif est-il affiché au coach ?
- Y a-t-il une limite de temps pour statuer ?

### Payload de `TeamEnrolled`

À figer avant de pouvoir implémenter la carte `32-team-enrollment.md` :

```
TeamEnrolled {
    event_id:        String,
    team_id:         String,
    space_id:        String,
    competition_id:  String,
    competition_name: String,   // pour enrichir la fiche équipe ?
    season_id:       String,
    season_name:     String,    // idem
}
```

### Questions ouvertes

- Une équipe peut-elle être inscrite dans **plusieurs compétitions** d'un même espace simultanément ?
- Si une compétition est annulée après inscription, le BC `competitions` publie-t-il un `TeamUnenrolled` ?

---

## Checklist (à compléter après raffinage)

- [ ] `EnrollmentRule` value object sur l'agrégat `Competition` (libre | validé)
- [ ] Règles d'éligibilité définies et modélisées
- [ ] Flux "accès validé" : UI demande coach + UI validation admin
- [ ] App event `TeamEnrolled` avec payload figé
- [ ] App event `TeamEnrollmentRejected` si nécessaire
- [ ] Publication depuis BC `competitions` vers app event bus
