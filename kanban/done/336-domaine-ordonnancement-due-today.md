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

- [x] `domain/notification_schedule.rs` : `due_today()`, `DueNotification`,
      `RoundRef`, les trois constantes de décalage
- [x] `domain/notification_delivery.rs` : `NotificationType`, `DeliveryKey`
- [x] Bibliothèque de dates : `time` 0.3, déjà dépendance
- [x] Les onze tests unitaires de la phase 6
- [x] Dont **« journée démarrant hier → rien »**, garde-fou unitaire de R9
- [x] Dont les deux tests sur `DeliveryKey` — ils testent une **forme**, et
      rougiront si quelqu'un retire un champ de la clé en croyant simplifier
- [x] `make check-arch`

## Ce qui a été fait

`domain/notification_delivery.rs` était **déjà écrit** : la carte 335 l'a créé
pour son dépôt, qui ne pouvait pas recevoir de clé autrement. Cette carte n'y
ajoute que les deux tests de forme.

Douze tests plutôt que onze : la spec liste onze cas, et le piège de la chaîne
vide en vaut un douzième pour la date limite — il n'était couvert que du côté
`date_start`.

Le test « journée à date fixe avec une `date_end` » est écrit plus fort que la
spec ne le demande : elle propose « `FixedDate` sans `date_end` », j'ai posé une
`date_end` **valide et à la bonne date**. C'est ce qui distingue vraiment la
règle — le type commande, pas la présence de la date — et c'est ce qui tomberait
si quelqu'un s'appuyait sur `date_end.is_some()`.

## Les deux garde-fous, vérifiés plutôt qu'affirmés

**R9.** En remplaçant `!=` par `>` dans la condition de la veille — la forme
qu'aurait une tolérance de rattrapage —,
`une_journee_ayant_demarre_hier_n_est_jamais_rattrapee` tombe. C'est exactement
ce que la spec attend de lui : « si un jour quelqu'un ajoute une tolérance de
rattrapage, ce test rougit avant tout le reste ».

**R2 et R3.** Les deux tests sur `DeliveryKey` ne testent pas du code mais une
**forme**. Ils exigent que `DeliveryKey` dérive `PartialEq`, ce qu'elle ne
faisait pas — ajouté ici.

## Détail d'exécution

Les tests du dépôt de la 335 échouent en `cargo test` nu (`DATABASE_URL must be
set`) : `#[sqlx::test]` la réclame, et c'est `make test` qui la pose. La règle
du projet — toujours `make test` — a une raison mécanique, pas seulement une
raison d'habitude.
