# Phase 6 — Domaine : le service d'envoi

**Entrée** : `05-use-cases.md`, validée. Récapitulatif des règles validé par
l'utilisateur.

## Récapitulatif exhaustif des règles métier

| | Règle | Unité |
|---|---|---|
| R1 | Une notification manquée est perdue, et journalisée | envoi |
| R2 | Un décalage de date réarme la notification | envoi |
| R3 | Idempotence par destinataire, garantie par la base | envoi |
| R4 | Tous les inscrits reçoivent l'email de journée, deux corps | envoi |
| R5 | Une notification inapplicable est grisée, avec son motif | configuration |
| R6 | Cochée puis rendue inapplicable, elle reste cochée | configuration |
| R7 | Le périmètre des destinataires est borné par l'espace | envoi |
| R8 | Saisons existantes éteintes, nouvelles allumées | configuration |
| R9 | L'activation n'est jamais rétroactive | envoi |
| R10 | Fuseau du serveur, le sélecteur disparaît | envoi |
| R11 | L'ouverture part à la validation, pas au cron | envoi |

Huit contraignent `envoi/`. Trois d'entre elles — R2, R7, R9 — sont déjà tenues
par des structures décidées aux phases 3 à 5, et non par du code à écrire ici.

## `due_today()` — le cœur de l'ordonnanceur

```rust
pub fn due_today(
    today:       &DateString,
    match_days:  &[MatchDay],
    invitations: Option<&CompetitionInvitations>,
    settings:    &CompetitionNotifications,
) -> Vec<DueNotification>;
```

Fonction **pure**, sans accès au journal — c'est ce qui tient R9 (phase 3). Elle
ne peut pas poser la question « qu'est-ce qui manque ? », faute d'avoir la donnée
pour y répondre.

### Les trois décalages sont des constantes de domaine

```rust
const EVE_OFFSET_DAYS:      i64 = 1;  // « la veille »
const CLOSING_OFFSET_DAYS:  i64 = 2;  // « deux jours avant la clôture »
const DEADLINE_OFFSET_DAYS: i64 = 3;  // « trois jours avant la date limite »
```

**Pas de configuration.** La maquette de date limite écrit « Plus que trois
jours » en toutes lettres, et celle de fin de journée annonce sa fenêtre. Un
réglage laisserait le nombre et le texte diverger sans que rien ne le signale —
un email affirmant « plus que trois jours » cinq jours avant l'échéance est pire
que pas d'email.

Si ces valeurs devaient un jour être réglables, le texte devrait le devenir dans
le même mouvement. Les changer ensemble ou pas du tout.

### Ce qu'elle décide

| Notification | Condition |
|---|---|
| `RoundEve` | réglage actif, journée non `Rest`, `date_start == today + 1` |
| `RoundClosing` | réglage actif, journée `TimeFrame`, `date_end == today + 2` |
| `RegistrationDeadline` | réglage actif, `registration_deadline == today + 3` |

`RegistrationOpen` n'en sort jamais : elle a son propre déclencheur (R11).

### Trois pièges, tous relevés en investigation

**Les journées `Rest` sont exclues.** `MatchDayType` a trois variantes, pas deux
— la table persistée connaît `rest`, que le type `ScheduledDate` de la structure
ignore. Une journée de repos n'a rien à annoncer.

**La chaîne vide n'est pas une date.** `DateString` est validée
`^(?:\d{4}-\d{2}-\d{2})?$` : la chaîne vide passe. Elle doit être traitée comme
absente, au même titre que `None`.

**`RoundClosing` exige `TimeFrame`, pas seulement une `date_end` non nulle.**
Les deux conditions coïncident aujourd'hui, mais s'appuyer sur la seconde ferait
dépendre une règle métier d'un invariant de persistance que rien ne garantit.

### `due_today()` et `applicability()` ne fusionnent pas

Elles se ressemblent assez pour qu'on soit tenté ; elles répondent à deux
questions différentes.

| | Question | Entrées | Sert |
|---|---|---|---|
| `applicability()` | « cela peut-il arriver un jour sur cette saison ? » | la **structure**, les invitations | l'écran de réglage |
| `due_today()` | « est-ce dû aujourd'hui ? » | les **journées persistées**, la date du jour | l'ordonnanceur |

Les entrées diffèrent — la structure décrit ce qui a été *voulu*, les journées ce
qui *existe*. Les fusionner obligerait l'écran de réglage à charger les journées,
et l'ordonnanceur à charger la structure, pour rien.

Leurs verdicts restent cohérents sans être partagés : `applicability()` rend
`NoTimeFrameRound` quand aucune journée n'a de fenêtre, et `due_today()`
n'émettra alors jamais de `RoundClosing`. La cohérence vient des mêmes faits, pas
d'un code commun.

### Deux notifications le même jour sont possibles, et acceptées

La journée 3 démarre demain pendant que la journée 2 clôt dans deux jours : le
coach reçoit deux emails. **Laissé tel quel** — ils ne disent pas la même chose,
et les fusionner demanderait une clé d'idempotence composite et un cinquième
gabarit, pour un cas peu fréquent.

## Bibliothèque de dates

`time` 0.3, déjà dépendance directe et utilisée par 49 fichiers, dont les
voisins de `DateString`.

Observation sans action : le projet dépend **aussi** de `chrono` 0.4, sur 14
fichiers concentrés dans `ranking`. Deux bibliothèques de date pour un projet
mériteraient une carte — pas celle-ci.

## Erreurs

`due_today()` ne retourne pas de `Result`. Une date illisible ne peut pas venir
du domaine : `DateString` valide son format à la construction. Le seul point de
défaillance est l'analyse de `today`, fourni par la CLI — traité **là-bas**, au
bord du système, pas ici.

Aucun ajout à `DomainError`.

## Tests unitaires

Sur `due_today()` — un par condition et par piège :

| Test | Attendu |
|---|---|
| journée démarrant demain, réglage actif | `RoundEve` |
| journée démarrant demain, réglage **inactif** | rien |
| journée démarrant après-demain | rien — pas de fenêtre glissante |
| journée démarrant **hier** | rien — R9, aucun regard en arrière |
| journée `Rest` démarrant demain | rien |
| journée `TimeFrame` clôturant dans deux jours | `RoundClosing` |
| journée `FixedDate` sans `date_end` | pas de `RoundClosing` |
| `date_start` à la chaîne vide | rien, comme si absente |
| date limite dans trois jours, réglage actif | `RegistrationDeadline` |
| date limite dans trois jours, pas d'invitations | rien |
| journée 3 démarrant demain **et** journée 2 clôturant dans deux jours | **deux** notifications |

Le test « journée démarrant hier » est le garde-fou de R9 au niveau unitaire.
C'est le seul endroit où la règle se vérifie sans monter une base : si un jour
quelqu'un ajoute une tolérance de rattrapage, ce test rougit avant tout le reste.

Sur `DeliveryKey` :

| Test | Attendu |
|---|---|
| deux clés ne différant que par `target_date` | **différentes** — R2, le décalage réarme |
| deux clés ne différant que par `coach_id` | différentes — R3, grain par destinataire |

Ces deux-là ne testent pas du code, ils testent une **forme**. Ils rougiront le
jour où quelqu'un retirera un champ de la clé en croyant simplifier — ce qui
casserait R2 ou R3 sans qu'aucun autre test ne bouge.

## Ce que cette phase laisse à la suivante

**Phase 7** — la migration et son index sur `COALESCE(round_id, '')`, les quatre
gabarits Askama, la sous-commande et ses arguments, le point d'accroche dans la
validation de l'étape 5, et les tests d'intégration et e2e.
