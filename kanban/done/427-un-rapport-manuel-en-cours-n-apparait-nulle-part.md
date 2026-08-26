# Un rapport de match manuel en cours n'apparaît nulle part

**Priorité : moyenne-haute** — un match démarré hors calendrier est invisible
pour toute la ligue jusqu'à sa publication
**Périmètre : la couche IO des BCs `match_report` et `competitions`**
**Dépend de :** rien

## Le symptôme

Un rapport de match créé **manuellement** — sans appariement préalable dans une
journée — n'apparaît pas dans l'onglet « Résultats » de la compétition tant
qu'il est en cours. Il n'y surgit qu'à la publication.

Pendant toute la saisie, personne ne voit que le match a commencé.

## Pourquoi

L'onglet lit `competition_match_display_proj`, une projection **dont la clef est
`pairing_id`** :

```sql
-- sql/match_days/list_resultats.sql
FROM competition_match_display_proj
WHERE season_id = $1 AND match_status IN ('in_progress', 'completed')
```

Un rapport manuel n'a pas d'appariement. Il n'a donc pas de ligne — sauf si
quelqu'un lui en fabrique une.

Trois listeners de `competitions` alimentent cette projection. **Deux savent le
faire, un ne sait pas.**

| Listener | Rapport manuel (`pairing_id: None`) |
|---|---|
| `match_report_published_listener` | `resolve_pairing_id` cherche un appariement, sinon **le crée** |
| `match_report_unpublished_listener` | même `resolve_pairing_id` |
| `match_report_confirmed_listener` | **abandonne en silence** |

La ligne fautive, `match_report_confirmed_listener.rs:36` :

```rust
let MatchReportAppEvent::MatchReportConfirmed {
    match_report_id,
    space_id,
    pairing_id: Some(pairing_id),   // ← un rapport manuel n'entre jamais ici
    ..
} = event
else {
    return;                          // sans une ligne de journal
};
```

Le motif filtre sur `Some`, et la branche `else` sort sans rien dire. C'est
exactement le cas qui a déjà été corrigé côté publication : le test de
régression `resolve_pairing_id_creates_a_real_pairing_and_projection_for_manual_reports`
(`match_report_published_listener.rs:455`) porte le commentaire

> Régression : un rapport manuel (`pairing_id: None`) n'avait jamais de ligne …
> "résultats".

**La correction n'a été portée que sur le chemin de publication.** Celui de la
confirmation — le seul qui écrit `in_progress` — a gardé l'ancienne garde.

## Ce qui manque pour corriger

`MatchReportConfirmed` ne porte pas de quoi fabriquer un appariement :

```rust
MatchReportConfirmed {
    event_id, match_report_id,
    home_team_id, away_team_id,
    space_id,
    pairing_id: Option<String>,
}                                   // ni season_id, ni round_id
```

Or `resolve_pairing_id` a besoin de `round_id` — pour chercher un appariement
existant sur `(round, home, away)`, puis pour rattacher celui qu'il crée. Et la
ligne de projection a besoin de `season_id`.

**L'agrégat les connaît tous les deux.** `MatchReportPreMatch` porte `season_id`
et `round_id` (`match_report_pre_match.rs:25-26`), et le formulaire de création
manuelle les saisit (`CreateMatchReportForm`). Ils sont disponibles au point
d'émission, ils ne sont simplement pas mis dans l'événement.

L'app event n'est **pas persisté** — `event_log` ne porte que des événements
domaine, par agrégat — donc l'enrichir ne demande aucun `#[serde(default)]` ni
aucune migration.

## La correction

1. **Enrichir `MatchReportConfirmed`** de `season_id` et `round_id`, aux deux
   points d'émission (`create_match_report_use_case.rs:106` et
   `update_match_selection_use_case.rs:101`) — les deux ont le `pre_match` sous
   la main.

2. **Factoriser `resolve_pairing_id`.** Il existe en deux exemplaires quasi
   identiques dans les listeners de publication et de dépublication. Le
   troisième listener ne doit pas en écrire un troisième : sortir la fonction
   dans un module partagé de `io/app_events/`, et l'appeler depuis les trois.

3. **Le listener de confirmation crée la ligne** au lieu d'abandonner : même
   résolution, puis l'`UPDATE … SET match_status = 'in_progress'` existant.

4. **Toute sortie anticipée journalise.** Le `else { return }` muet est ce qui a
   rendu ce défaut invisible pendant des mois : rien dans le journal ne dit
   qu'un événement a été reçu et ignoré.

## La conséquence à traiter en même temps

Créer l'appariement **à la confirmation** et non plus seulement à la publication
change la durée de vie de l'objet : un rapport abandonné ou annulé laisserait
désormais un appariement orphelin au calendrier.

Or **`MatchReportCancelled` n'a aucun listener dans `competitions`.** L'app event
est bien publié (`match_report/io/app_events/app_event_publisher.rs:147`), mais
personne ne l'écoute.

C'est déjà un défaut aujourd'hui pour les matchs **programmés** : annuler un
rapport laisse son appariement en `in_progress` pour toujours. La correction
ci-dessus le rend seulement plus visible.

**Le listener d'annulation fait donc partie de cette carte** : il remet
l'appariement programmé en `upcoming`, et **supprime** celui qui avait été créé
pour un rapport manuel — il n'existait que pour lui.

## Mesure

Sur la base de développement le 2026-08-26 :

```
origin=Pairing, avec appariement   7304
origin=Manual,  sans appariement    695     ← aucun n'a de ligne d'affichage
```

Les 695 se répartissent en 386 `Published`, 208 `PreMatch`, 101
`ReadyToPublish`. **Attention à l'interprétation** : ces données sont pour
l'essentiel générées pour la démonstration et insérées hors de l'application,
donc l'absence de ligne pour les 386 publiés ne prouve pas que le chemin de
publication est cassé. Ce qui est certain vient de la lecture du code, pas de ce
comptage : le chemin de confirmation, lui, ne peut rien créer.

## Tests

**Unitaires**
- `un_rapport_manuel_confirme_obtient_un_appariement_et_une_ligne` — le cas de
  la carte.
- `un_rapport_manuel_reconfirme_ne_cree_pas_un_second_appariement` —
  l'idempotence que `resolve_pairing_id` assure déjà côté publication, et qu'il
  faut préserver en la factorisant.
- `un_rapport_programme_confirme_ne_cree_aucun_appariement` — le chemin normal
  ne change pas.
- `l_annulation_d_un_rapport_manuel_supprime_son_appariement`.
- `l_annulation_d_un_rapport_programme_le_remet_en_upcoming`.

**E2E**
- Créer un rapport manuellement, puis vérifier qu'il apparaît dans l'onglet
  Résultats **avant** publication, avec le statut « en cours ».

C'est le test qui décrit le bug tel qu'il a été constaté, et aucun test unitaire
ne peut le remplacer : il traverse deux BCs et un bus d'événements.

## Checklist

- [ ] `MatchReportConfirmed` porte `season_id` et `round_id`
- [ ] Les deux points d'émission les renseignent
- [ ] `resolve_pairing_id` factorisé, trois appelants
- [ ] Le listener de confirmation crée la ligne
- [ ] Toute sortie anticipée des trois listeners journalise
- [ ] Un listener `MatchReportCancelled` dans `competitions`
- [ ] Les cinq tests unitaires et le test e2e
- [ ] `make lint && make test && make check-arch && make e2e`
