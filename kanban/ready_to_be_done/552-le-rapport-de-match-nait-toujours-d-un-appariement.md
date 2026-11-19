# Le rapport de match naît toujours d'un appariement

**Priorité : haute — suite de l'incident G. B. L. R du 15 septembre 2026**
**Épic :** aucune — un renversement de flux, livrable d'un bloc
**Dépend de :** carte 551 (l'invariant de journée, que l'écran de création doit rencontrer)
**S'appuie sur :** carte 550, livrée — elle fournit l'option `autorise_hors_calendrier`, sa méthode sur `ICompetitionDataPort`, et la vue figée de la phase 1 que cette carte généralise
**Fichiers :**
`src/app/match_report/io/web/match_selection_controller.rs` *(reprise du code de la 550)*,
`src/app/match_report/io/web/templates/match-selection.html` *(reprise du code de la 550)*,
`src/app/match_report/use_cases/update_match_selection_use_case.rs`,
`src/app/match_report/use_cases/create_match_report_use_case.rs`,
`src/app/match_report/use_cases/cancel_match_report_use_case.rs` *(supprimé)*,
`src/app/match_report/io/web/cancel_match_report_controller.rs` *(supprimé)*,
`src/app/match_report/io/web/templates/cancel-button.html` *(supprimé)*,
`src/app/match_report/domain/value_objects.rs` *(`MatchReportOrigin` supprimé)*,
`src/app/match_report/domain/events.rs`,
`src/app/match_report/io/repository/match_report_repository.rs`,
`src/app/match_report/routes.rs`, `src/app/match_report/router.rs`,
`src/app/competitions/io/app_events/match_report_confirmed_listener.rs`,
`src/app/competitions/io/app_events/appariement.rs`,
`src/app/competitions/io/app_events/match_report_cancelled_listener.rs`,
`src/infrastructure/data_migrations/m003_rapports_delies.rs` *(nouveau)*,
`src/infrastructure/data_migrations/mod.rs`,
`tests/e2e/` *(nouveau test)*, `tests/impact-map.toml`

## L'objectif

Un seul sens de flux : **appariement → rapport**, toujours. Le rapport ne naît
pas seul et ne meurt pas seul.

## Ce qui l'a fait naître

L'incident G. B. L. R du 15 septembre, et la mesure qui a suivi : **16 rapports
vivants dont l'agrégat ignore son appariement**.

Le chemin manuel remonte le courant. Un rapport « hors calendrier » naît sans
appariement ; `competitions` lui en fabrique un après coup
(`resoudre_ou_creer_appariement`) et **ne le lui dit jamais**. Son
`MatchReportCreated` porte `pairing_id: null` à vie, et
`match_report_proj.pairing_id` reste NULL.

Toutes les recherches inverses pairing → rapport sont alors aveugles :

| recherche | conséquence observée |
|---|---|
| `find_id_by_pairing` (`pairing_deleted_listener`) | l'appariement supprimé n'annule pas le rapport — **rapport orphelin**, invisible partout, équipes bloquées en `MatchReporting` |
| `find_phases_by_pairings` (garde-fou de suppression) | un rapport manuel **publié** ne protège pas son appariement : le match disparaît du calendrier et des résultats **en restant au classement** |

Le premier cas s'est produit le 15 septembre à 13:41 et n'a pas laissé une seule
ligne de journal : le listener sort sur `Ok(None) => return`.

## Le changement

| | |
|---|---|
| **Hors calendrier** | l'écran crée d'abord l'appariement (équipe A, équipe B, journée), puis enchaîne sur le chemin normal. Il rencontre l'invariant de la carte 551 |
| **Couple déjà programmé** | redirection vers le rapport existant, **pas** un refus — répondre « ce match existe déjà » à qui veut précisément le saisir est une impasse. C'est déjà ce que fait `confirm_existing` |
| **`MatchReportOrigin`** | supprimé. Hors tests, il ne sert qu'à **une ligne** (`create_match_report_use_case.rs:75`, l'auto-confirmation du chemin manuel), qui disparaît avec le chemin |
| **Annulation autonome** | supprimée : use case, contrôleur, route, bouton |
| **Suppression** | seul l'appariement se supprime ; il emporte son rapport ; refusée si le rapport est publié |
| **Phase 1** | devient une **confirmation pour tout le monde**, admin compris : un rapport né d'un appariement a ses équipes fixées, il n'y a plus rien à y choisir |
| **La garde de la 550** | se déplace de `create_match_report` vers la création de l'appariement hors calendrier — seul point d'entrée qui subsiste |

Le garde-fou « publié » **redevient effectif sans une ligne de code** :
`pairing_id` étant désormais toujours renseigné, `find_phases_by_pairings` voit
enfin les rapports manuels.

## Ce que la carte 550 laisse à reprendre

La 550 a figé la phase 1 **pour les non-admins des compétitions qui interdisent
le hors-calendrier**. Cette carte supprime la distinction : il n'y a plus de cas
où les équipes d'un rapport se choisissent.

**Le travail n'est pas perdu, il devient le cas général.** `SelectionFigeeVm` et
le bloc de gabarit qui affiche les quatre noms en clair sont exactement ce que
la phase 1 doit montrer désormais à tous. Le champ cesse d'être une `Option`.

Ce qui disparaît, ce sont les mécanismes conditionnels qui l'entouraient, faute
de cas à distinguer :

| | |
|---|---|
| `selection_figee()` | le prédicat à deux conditions — plus rien à décider |
| `equipes_retenues()` | les équipes du formulaire ne sont jamais retenues |
| la branche `{% else %}` de `match-selection.html` | les deux widgets de sélection quittent cet écran |
| `update_match_selection` | se réduit à confirmer `Draft` → `PreMatch` |

Le commentaire de `new_match_report` — « Jamais figée : la garde de
`create_match_report` interdit déjà d'y arriver » — **devient faux** quand la
garde se déplace. Le corriger fait partie de la carte.

⚠️ **Ne pas emporter** `ICompetitionDataPort::autorise_hors_calendrier` ni
`IHorsCalendrierPort` : l'option reste, le menu conditionnel reste, seule change
la place où la garde s'applique. Règle 4 — lister les consommateurs avant de
supprimer.

## Pourquoi l'annulation autonome disparaît

Elle servait à sortir d'un rapport **ouvert par erreur** : une fois en
`PreMatch`, les équipes ne sont plus modifiables (`update_match_selection`
n'accepte que `Draft`), et l'annulation était la seule issue. Trois coachs s'en
sont servis en production, contre 817 annulations en cascade.

Ce besoin disparaît avec le chemin manuel : le rapport n'est plus *ouvert* par
un geste, il existe dès que l'appariement existe. L'erreur ne peut donc plus
porter que sur l'appariement, et le remède est de supprimer l'appariement.

**Décision assumée** : tout abandon passe désormais par la suppression de
l'appariement, aujourd'hui réservée aux commissaires. Reste à trancher si un
coach peut supprimer l'appariement **qu'il a lui-même créé** tant que le rapport
n'est pas publié — sinon la moindre erreur de saisie hors calendrier mobilise un
commissaire. Par défaut, l'implémentation garde le statu quo (admin seulement).

## Ce qui tombe au passage

`MatchReportCancelled` n'étant plus émis que par la cascade `PairingDeleted`,
`competitions::match_report_cancelled_listener` semble perdre tout objet : sa
branche `supprimer_appariement_manuel` n'a plus de cas (plus de rapport sans
appariement), et sa branche `remettre_a_venir` agirait sur une ligne déjà
supprimée par `delete_pairing`.

**À vérifier, pas à supposer** — règle 4 : lister les consommateurs avant de
supprimer.

## La reprise de données — migration `m003`

Elle se joue à la livraison. À l'issue, **aucun orphelin dans les deux sens**.

16 rapports vivants déliés, quatre traitements :

| n | situation | action |
|---|---|---|
| 3 | appariement connu de la seule projection (`competition_match_display_proj.match_report_id`) | poser le lien |
| 11 | appariement rapprochable par (journée, équipes) | poser le lien |
| 1 | `01M1PRBRF44GPZFB35GB7GR624` — J2, espace « Ligue », aucun conflit | créer l'appariement, poser le lien |
| 1 | `01M2JDKAC0HS6Y3P82S8056R8N` — J15 G. B. L. R | **annuler le rapport** |

**Il n'y a aucun appariement orphelin.** Les 11 qui en avaient l'air
appartiennent aux rapports ci-dessus : le rapprochement par (journée, équipes)
les associe un pour un. Une fois les liens posés, le compte tombe à zéro sans
qu'on supprime quoi que ce soit.

### Trois points de méthode

**Le rapprochement teste les deux camps.** Rien ne garantit que le rapport et
l'appariement aient retenu le même domicile ; ne comparer que `home_team_id`
manquerait la moitié des cas.

**Un `UPDATE` sur `match_report_proj` ne suffit pas.** Le `pairing_id` n'y est
écrit qu'à l'INSERT, depuis `MatchReportCreated` où il vaut `null`. Un rebuild
de projection effacerait la correction, et le défaut reviendrait sans prévenir.
La reprise doit **appender un événement** à l'event store de chaque rapport, que
la projection consomme.

**Le cas J15 ne peut pas retrouver d'appariement.** Recréer RUR–Pillards ferait
jouer les deux équipes deux fois cette journée-là : Rainbow Unicorn Riders
affronte Ork'Lympic Rusé, les Pillards affrontent Lady's Ghosts. Le script
violerait l'invariant de la carte 551 sur sa première ligne. Le rapport est donc
**annulé** — il ne contient aucune saisie (2 événements : création et
confirmation de sélection), et son annulation libère les deux équipes de leur
phase `MatchReporting`.

Les deux appariements légitimes de la J15 **ne sont pas touchés**. Le fantôme
RUR–Pillards n'existe plus depuis le 15/09 à 13:41.

## Tests

- **Unitaire** : la création hors calendrier produit l'appariement puis le
  rapport, et le rapport porte son `pairing_id`.
- **Unitaire** : supprimer un appariement dont le rapport est publié est refusé
  — y compris pour un rapport créé hors calendrier, le cas qui échouait.
- **E2E** : un coach saisit une rencontre hors calendrier ; elle apparaît au
  calendrier, puis aux résultats une fois la saisie commencée. Un commissaire
  supprime l'appariement ; le rapport disparaît et les deux équipes sont
  libérées.
- **Migration** : les deux requêtes de contrôle renvoient zéro ligne après
  exécution.

## État au 16 septembre 2026 — le cœur est livré, la carte ne l'est pas

Le renversement du flux et la reprise de données sont en place et vérifiés ; il
reste trois choses, d'où le maintien de cette carte en `ready_to_be_done`.

**Livré :**

- l'appariement créé avant le rapport, via le port `creer_appariement` dont
  l'adapter appelle le même use case que l'ajout par un commissaire ;
- l'auto-confirmation retirée : la phase 1 est une confirmation pour tous ;
- l'annulation autonome retirée — use case, contrôleur, route, bouton sur cinq
  écrans, et son test e2e ;
- la migration `m003`, **exécutée sur la base de production importée** : 16
  rapports déliés traités, les deux contrôles de fin rendent zéro, le rapport
  de la J15 annulé et ses deux équipes libérées.

**Reste :**

- **`MatchReportOrigin` n'est pas supprimé.** Il n'a plus d'usage fonctionnel
  mais vit dans `MatchReportCreated`, donc dans l'event store, et le retirer
  touche une vingtaine de fichiers de tests.
- **`competitions::match_report_cancelled_listener` n'est pas supprimé.** La
  section ci-dessus le dit « à vérifier plutôt qu'à supposer » — la vérification
  n'a pas été faite.
- **Aucun test e2e du parcours hors calendrier**, ni test unitaire de `m003`.
  Celle-ci est validée sur données réelles, ce qui ne se rejoue pas en CI.

## Terminé quand

Les deux contrôles passent sur la base de production reprise :

```sql
-- aucun rapport vivant sans appariement
SELECT count(*) FROM match_report_proj
WHERE phase NOT IN ('Cancelled') AND pairing_id IS NULL;

-- aucun appariement sans rapport vivant
SELECT count(*) FROM competition_match_day_pairings p
LEFT JOIN match_report_proj m ON m.pairing_id = p.id AND m.phase <> 'Cancelled'
WHERE m.match_report_id IS NULL;
```

Et un coach qui saisit un match hors calendrier voit sa rencontre apparaître au
calendrier puis aux résultats, sans qu'aucun appariement ne soit fabriqué dans
son dos.
