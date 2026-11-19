# Un sens de flux par cas métier

**Priorité : haute — un coach peut aujourd'hui créer deux rapports pour une même rencontre**
**Épic :** aucune — une clarification de mécanisme, livrable d'un bloc
**Dépend de :** rien (la carte 551 est livrée)
**Schéma :** https://claude.ai/artifact/REJU4L97fuC6pYZvcLs5og
**Fichiers :**
`src/app/competitions/use_cases/admin/add_match_use_case.rs`,
`src/app/competitions/domain/domain_event.rs`,
`src/app/competitions/io/app_events/app_event_publisher.rs`,
`src/app/competitions/io/app_events/appariement.rs`,
`src/app/competitions/io/app_events/match_report_confirmed_listener.rs`,
`src/app/competitions/io/app_events/match_report_published_listener.rs`,
`src/app/match_report/ports.rs`,
`src/infrastructure/match_report/competition_data_adapter.rs`,
`src/app/match_report/io/web/match_selection_controller.rs`,
`src/app/match_report/use_cases/create_match_report_use_case.rs`,
`src/infrastructure/data_migrations/m004_rapports_en_double.rs` *(nouveau)*,
`src/infrastructure/data_migrations/mod.rs`

## L'objectif

**Un créateur de rapport par cas métier, et un seul.** Le calendrier passe par
l'événement, le hors calendrier par l'appel direct. Aucun des deux n'attend
l'autre.

## Ce qui l'a fait naître

La carte 552 a renversé le flux du hors calendrier sans trancher **qui** crée le
rapport quand les deux chemins se rejoignent. Résultat : deux acteurs le créent.

```
POST /match-report/new
  └─ le contrôleur fait créer l'appariement      (appel direct, port)
       ├─ PairingCreated ─► pairing_created_listener ─► rapport ①
       └─ le contrôleur ────────────────────────────► rapport ②
```

Les deux se dédupliquent en relisant la base ; chacun lit avant que l'autre
n'écrive. **38 appariements portent deux rapports vivants** après une seule
exécution de la suite e2e.

C'est ce qui rendait la suite illisible : selon celui qui gagnait, un test
manipulait le rapport ou son jumeau — huit échecs qui apparaissaient et
disparaissaient d'une passe à l'autre sans qu'aucune dépendance ne change.

### L'asymétrie qui la fabrique

L'appel au port est **synchrone**, ses conséquences ne le sont pas. Le
contrôleur récupère le `pairing_id` en microsecondes et croit l'opération
finie, alors que `PairingCreated` vient de partir sur deux bus vers un listener
qui, lui, créera un rapport.

## Le changement

| cas | qui crée l'appariement | qui crée le rapport | sens |
|---|---|---|---|
| **calendrier** | `competitions`, depuis l'administration | le listener, sur `PairingCreated` | événement — personne n'attend |
| **hors calendrier** | `competitions`, appelé par le port | **le contrôleur**, au retour de l'appel | appel direct — réponse immédiate |

Un sens par cas métier. Pas de course, et surtout **pas d'attente non
déterministe** : l'alternative — faire attendre le contrôleur que le listener
ait fini — ferait dépendre une réponse HTTP de l'ordonnancement d'une tâche.

## 1 · Séparer le cœur de son annonce

Le port ne doit pas émettre `PairingCreated`, sans quoi le listener crée le
rapport que le contrôleur s'apprête à créer. Mais il doit appliquer **la même
règle** — l'invariant de la carte 551 — sans la dupliquer.

```
verifier_et_enregistrer(journée, équipes)      ← l'invariant + save_pairing
    ├── add_match_use_case  → puis emettre(PairingCreated)              administration
    └── creer_hors_calendrier → puis emettre(OutOfSchedulePairingCreated)   le port
```

Une seule implémentation de la règle, deux enveloppes qui ne diffèrent que par
l'annonce.

**`competitions` reste souverain sur les appariements** dans les deux cas : le
hors calendrier ne contourne pas sa règle, il la consulte.

## 2 · `OutOfSchedulePairingCreated`

Un **domain event** de `competitions`, émis sur le bus interne et persisté par
`event_log_feeder`. Il dit un fait du domaine — une rencontre existe qui n'était
pas programmée — et non l'origine du déclencheur : `PairingCreatedFromMatchReport`
aurait été le contre-exemple que la convention proscrit.

**Il remplace `PairingCreated` sur ce chemin, il ne s'y ajoute pas.**

**Il ne sort pas du BC.** Il est donc nommé dans le bras groupé de
`to_app_event`, jamais avalé par un joker :

```rust
| CompetitionsDomainEvent::OutOfSchedulePairingCreated { .. } => None,
```

Sans cette ligne explicite, ce serait la « forme B » que le `CLAUDE.md` décrit —
un événement émis que personne ne reçoit. Avec elle, c'est une décision.

### Ce qu'il apporte

L'incident du 15 septembre a demandé de croiser des ULID et des horodatages pour
établir qu'un appariement avait été fabriqué hors calendrier. Avec cet
événement, c'est un `grep` dans `event_log`.

### Ce que la vérification a montré

`pairing_created_listener` est le **seul** abonné à `PairingCreated`. Ne pas
l'émettre n'a donc aucun effet collatéral — et le commentaire d'`appariement.rs`
qui justifiait cette émission « pour la cohérence du système (ex. suivi des
équipes déjà affrontées) » est **faux** : la génération lit la base, pas les
événements.

## 3 · L'aiguillage remonte dans le contrôleur

Trois cas, décidés **là où on les lit**, et non enfouis dans l'adapter :

| situation | traitement |
|---|---|
| le couple a déjà un appariement ce jour-là | c'est un match du calendrier — on ouvre son rapport existant |
| une des deux équipes est prise par un autre adversaire | **erreur** : l'invariant parle, et le coach lit pourquoi |
| aucune des deux n'est prise | hors calendrier : créer l'appariement, puis le rapport |

Le premier cas existe déjà, mais caché dans `appariement_du_couple` de
l'adapter, où rien ne le nomme. Il devient un aiguillage visible.

## 4 · La déduplication tombe

Elle n'existait que pour départager deux créateurs. Avec un seul, elle n'a plus
d'objet :

- `confirm_existing` et son appel dans `create_match_report_use_case` ;
- la relecture idempotente de `ouvrir_le_rapport`, posée pour masquer le `500`
  que la course produisait.

`find_id_by_round_and_teams` **reste**, mais change de rôle : ce n'est plus une
déduplication dans le use case, c'est l'aiguillage du contrôleur.

## 5 · Débrancher l'ancien sens

`resoudre_ou_creer_appariement` fabrique un appariement pour un rapport qui n'en
a pas. Avec la carte 552, ce cas n'existe plus. Ses appels vivent dans
`match_report_confirmed_listener` et `match_report_published_listener`.

⚠️ **Huit sites d'appel, pas deux.** Six sont dans le listener de publication.
Instruire ce qu'ils assurent avant de couper — règle 4. `trouver_appariement`,
la recherche seule, reste : la dépublication en a besoin.

## 6 · Reprendre les 38 doublons — migration `m004`

Pour chaque appariement à deux rapports vivants : garder **le plus avancé**
(`Published` > `ReadyToPublish` > `PreMatch` > `Draft`), annuler l'autre. À
égalité de phase, garder le plus ancien — c'est celui que les écrans ont montré.

Contrôle de fin : aucun appariement ne porte plus d'un rapport vivant.

## Tests

**Unitaires**

| | |
|---|---|
| `verifier_et_enregistrer` | la même règle refuse dans les deux enveloppes — un test par chemin, sinon la duplication reviendra sans qu'on le voie |
| l'annonce | `add_match_use_case` émet `PairingCreated`, le port émet `OutOfSchedulePairingCreated` — et **jamais l'inverse** |
| l'aiguillage | les trois cas, dont celui de l'erreur avec le nom de l'adversaire |

**E2E** — un coach saisit une rencontre hors calendrier : **un** rapport en base,
un seul appariement, et l'écran de saisie s'ouvre. La contre-épreuve compte les
rapports, sans quoi le test passerait avec deux.

## Terminé quand

```sql
-- aucun appariement ne porte deux rapports vivants
SELECT count(*) FROM (
  SELECT pairing_id FROM match_report_proj
  WHERE phase <> 'Cancelled' AND pairing_id IS NOT NULL
  GROUP BY 1 HAVING count(*) > 1) x;
```

Et la suite e2e rend le même verdict deux fois de suite sur une base neuve — ce
qu'elle ne fait pas aujourd'hui.
