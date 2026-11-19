# MatchDay expose ses appariements à qui veut les modifier

**Priorité : basse** — aucun défaut constaté, mais un invariant contournable
**Épic :** aucune
**Dépend de :** l'appariement des présents (`docs/specs/appariement-des-presents/`)
**Fichiers :** `src/app/competitions/domain/match_day.rs` et ses consommateurs

## Le constat (2026-10-28)

Tous les champs de `MatchDay` sont `pub`, `pairings: Vec<Pairing>` compris.
Relevé à la phase 6 de l'appariement des présents : `accept_draw` y pose les
invariants d'un tirage — journée vide, équipe une seule fois, jamais deux
équipes d'un même coach, jamais deux poules mêlées — et `accueillir` (carte
557) celui d'une équipe engagée une seule fois par journée.

Rien n'empêche pourtant d'écrire `match_day.pairings.push(…)` : les deux
méthodes gardent une porte que le champ laisse ouverte.

Accès directs à `.pairings` hors de `match_day.rs`, au 2026-10-28 :

| Fichier | Accès |
|---|---|
| `domain/notification_schedule.rs` | 1 |
| `io/web/admin/admin_scope.rs` | 1 |
| `io/web/admin/schedule_widgets.rs` | 2 |
| `io/web/widgets/move_pairing_widget.rs` | 1 |
| `use_cases/admin/delete_pairing_use_case.rs` | 2 |
| `use_cases/admin/generate_pairings.rs` | 2 |
| `use_cases/admin/move_pairing_use_case.rs` | 1 |
| `use_cases/notification_recipients.rs` | 1 |

Le décompte est à refaire avant de commencer : `generate_pairings.rs` change
avec l'appariement des présents.

## Ce qu'il faudrait

Un champ privé, une lecture `pairings(&self) -> &[Pairing]`, et un
constructeur de réhydratation pour le dépôt — qui aujourd'hui construit
l'agrégat champ par champ.

## Questions ouvertes

- Les autres champs (`name`, `day_type`, dates, `position`) sont-ils dans le
  même cas ? Les modifier ne contourne aucun invariant connu ; les encapsuler
  tous élargit la carte à tout ce qui lit une journée.
- Les accès en lecture seule (la majorité) passent-ils par `pairings()`, ou
  certains révèlent-ils une question que le domaine devrait savoir répondre
  lui-même (« cette équipe joue-t-elle cette journée ? » existe déjà :
  `engagement_existant`) ?
