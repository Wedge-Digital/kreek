# Le plafond de participants n'a jamais servi

**Priorité : moyenne** — un réglage mort qui produit un e-mail incomplet
**Périmètre : l'écran et le modèle**
**Dépend de :** rien. La 414 et l'onglet Paramètres l'ont fait remonter, aucune
des deux ne l'attend.

## Le constat

`CompetitionInvitations.max_participants: Option<u32>` se saisit à l'étape 4 du
magicien, s'affiche dans deux récapitulatifs, et remplit une ligne d'e-mail.

**Il ne règle rien.** `team_enrollment.rs` ne le lit pas : aucune inscription
n'a jamais été refusée pour cause de compétition pleine. C'est un nombre
d'affichage déguisé en réglage, et l'onglet Paramètres a refusé de le proposer
pour cette raison (`docs/specs/modifier-une-competition/`).

**Et personne ne l'a jamais posé.** Mesuré le 2026-08-26 :

```sql
select coalesce(invitations->>'max_participants','(absent)'), count(*)
from competition_seasons group by 1;
  (absent) | 1874
```

Mille huit cent soixante-quatorze saisons, zéro plafond. Le champ existe depuis
l'origine du magicien et n'a **jamais** été rempli une seule fois.

## Le défaut visible

`assets/templates/emails/fr_FR/competition_registration_deadline.html:180`

```html
<div class="info-row">
  <span class="info-label">Places restantes</span>
  <span class="info-value">{{ remaining_slots }}</span>
</div>
```

**Aucun `{% if %}` dans ce gabarit** — il n'en contient pas un seul. La ligne
part donc toujours, et comme aucune saison n'a de plafond,
`places_restantes()` rend `String::new()` à chaque fois
(`send_due_notifications_use_case.rs:237`).

**Tout e-mail de date limite jamais envoyé porte donc une ligne « Places
restantes » vide.**

Le commentaire de `places_restantes` affirme le contraire :

> « on rend une chaîne vide **et** le gabarit ne montre alors pas la ligne —
> c'est mieux que d'annoncer "il reste  places" »

La première moitié est vraie, la seconde est fausse. Le gabarit ne sait pas
masquer quoi que ce soit. C'est exactement le défaut que le commentaire
prétendait éviter, écrit à côté du code qui le produit.

## Ce que la carte fait

**Retirer le plafond partout, et la ligne d'e-mail avec lui.**

| Où | Quoi |
|---|---|
| `domain/competition_invitations.rs:48` | le champ `max_participants` |
| `new-competition-phase-4.html` | l'état `maxParticipants`, le champ de saisie, le POST (lignes 157, 188, 245, 261, 298, 311) |
| `new_competition_phase_5.rs:228` | le récapitulatif « N / M places » |
| `io/web/admin/summary_tab.rs:351` | idem, onglet Résumé |
| `use_cases/admin/dashboard_query.rs` | **rien à faire** : l'onglet Tableau de bord disparaît avec l'onglet Paramètres |
| `send_due_notifications_use_case.rs:237` | `places_restantes()` et son appel |
| `io/email/notification_emails.rs:97` | le champ `remaining_slots` de `RegistrationDeadlineEmail` |
| `competition_registration_deadline.html:179-183` | le bloc `info-row` **et son `<hr class="info-sep">`** |
| chaînes JSON de test | `competition_invitations.rs`, `notification_schedule.rs`, `competition_notifications.rs` |

Le `<hr>` compte : le laisser produirait deux séparateurs consécutifs en bas du
bloc d'informations.

## Ce qui rend le retrait sûr

**Aucune migration de données.** `serde` ignore les champs inconnus par défaut,
et `deny_unknown_fields` n'apparaît nulle part dans le projet. Les 1874
documents `invitations` déjà écrits se désérialiseront sans lui — et de toute
façon aucun ne le porte avec une valeur.

**Les récapitulatifs se replient déjà.** Les deux sites de calcul sont écrits
`if let Some(max) = inv.max_participants { … } else { … }` : la branche `else`
est celle qui s'exécute depuis toujours, c'est la seule qui reste.

## Ce que la carte ne fait pas

**Elle n'implémente pas de limite de places.** Si un plafond opposable est
souhaité un jour, c'est une fonctionnalité : un refus à l'inscription, un
message au coach, une décision sur les inscriptions en attente. Retirer un
réglage mort et écrire une règle vivante sont deux travaux différents, et les
confondre ferait passer le second pour un détail du premier.

## Tests

- **Unitaire** : la désérialisation d'un `invitations` portant encore
  `max_participants` réussit et l'ignore — c'est ce qui prouve qu'aucune
  migration n'est due.
- **Rendu d'e-mail** : le gabarit de date limite ne contient plus ni la ligne
  ni son séparateur. Le test de non-vacuité des variables
  (`notification_emails.rs`) doit rester vert sans `remaining_slots`.
- **E2E** : l'étape 4 du magicien s'enchaîne sans le champ, et l'étape 5
  n'affiche plus de ligne de places.
