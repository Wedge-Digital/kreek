# Le rapport de match naît toujours d'un appariement

**Priorité : haute — suite de l'incident G. B. L. R du 15 septembre 2026**
**Épic :** aucune — un renversement de flux, livrable d'un bloc
**Dépend de :** carte 551 (l'invariant de journée, que l'écran de création doit rencontrer)
**Fichiers :**
`src/app/match_report/io/web/match_selection_controller.rs`,
`src/app/match_report/io/web/templates/match-selection.html`,
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

Le garde-fou « publié » **redevient effectif sans une ligne de code** :
`pairing_id` étant désormais toujours renseigné, `find_phases_by_pairings` voit
enfin les rapports manuels.

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
