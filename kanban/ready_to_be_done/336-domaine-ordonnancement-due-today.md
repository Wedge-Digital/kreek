# Domaine de l'ordonnancement — `due_today()`

**Spec :** `docs/specs/notifications/envoi/06-domaine.md`
**Dépend de :** 331 *(pour `CompetitionNotifications`)*
**Ouvre :** 340

## Objectif

La fonction qui décide ce qui part aujourd'hui. Pure, testable sans base ni
réseau.

## Conception

```rust
pub fn due_today(
    today:       &DateString,
    match_days:  &[MatchDay],
    invitations: Option<&CompetitionInvitations>,
    settings:    &CompetitionNotifications,
) -> Vec<DueNotification>;
```

**Cette signature est ce qui tient R9.** La fonction n'a aucun accès au journal
d'envois : elle ne peut pas poser la question « qu'est-ce qui manque ? », même si
quelqu'un le voulait. Ne pas lui ajouter de paramètre qui l'y autoriserait.

### Les trois décalages sont des constantes, pas de la configuration

Veille = `today + 1`, clôture = `today + 2`, date limite = `today + 3`. La
maquette écrit « Plus que trois jours » en toutes lettres : un réglage laisserait
le nombre et le texte diverger. Les changer ensemble ou pas du tout.

### Trois pièges

- Les journées `Rest` sont exclues — `MatchDayType` a **trois** variantes.
- La chaîne vide n'est pas une date : `DateString` l'autorise, il faut la traiter
  comme absente.
- `RoundClosing` exige le type `TimeFrame`, **pas** simplement une `date_end` non
  nulle : s'appuyer sur la seconde ferait dépendre une règle métier d'un
  invariant de persistance que rien ne garantit.

### Ne pas fusionner avec `applicability()`

L'une demande « cela peut-il arriver un jour ? » à partir de la **structure**,
pour l'écran de réglage ; l'autre « est-ce dû aujourd'hui ? » à partir des
**journées persistées**. La structure décrit ce qui a été voulu, les journées ce
qui existe.

## Checklist

- [ ] `domain/notification_schedule.rs` : `due_today()`, `DueNotification`,
      `RoundRef`, les trois constantes de décalage
- [ ] `domain/notification_delivery.rs` : `NotificationType`, `DeliveryKey`
- [ ] Bibliothèque de dates : `time` 0.3, déjà dépendance
- [ ] Les onze tests unitaires de la phase 6
- [ ] Dont **« journée démarrant hier → rien »**, garde-fou unitaire de R9
- [ ] Dont les deux tests sur `DeliveryKey` — ils testent une **forme**, et
      rougiront si quelqu'un retire un champ de la clé en croyant simplifier
- [ ] `make check-arch`
