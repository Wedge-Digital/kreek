# Player match impact — Spec index

Impact des rapports de match sur les joueurs : stats de carrière, SPP, blessures et
disponibilité. Émission d'une famille d'app events (« PlayerReportEvents ») par le
BC `match_report` à la publication d'un rapport, consommés par le BC `players`.

Pas de nouvelle page front — la donnée alimentera la fiche joueur déjà maquettée
(`assets/rawpages/html/app-player-detail-readonly.html`), câblage différé à une
carte ultérieure. Les Phases 1/2 (maquettes, architecture front) sont donc sans
objet ici ; la Phase 3 (architecture back) est traitée en version allégée dans ce
même document faute de widgets/routes HTTP (feature 100% événementielle inter-BC).

Le statut « fiche joueur » (`ReadyToPlay | SpendingSpp | Customizing`, piloté par les
boutons de la fiche joueur) et le statut `Retired` (retraite définitive, déclenchée
par la vente d'un joueur en phase de renvois post-match) sont explicitement **hors
périmètre** de cette feature.

## Pages

| Page | Front | Back | DTOs | Use cases | Domaine | Intégration | Cartes |
|---|---|---|---|---|---|---|---|
| player-report-events | ➖ (pas de front) | ➖ (pas de widget/route HTTP, traité dans 07) | ✅ | ➖ (pas de use case dédié, méthodes domaine appelées directement par les listeners) | ✅ | ✅ | ✅ |

*(DTOs = contrats des `PlayerReportEvents`, phase 4, figés dans la discussion précédant `06-domaine.md` ; Domaine = phase 6, validé.)*

## Règles métier transverses (identifiées en phase 1-6)

### Portée des événements

- Seules les actions de joueurs `Regular` (avec `player_id` stable) produisent des `PlayerReportEvents`. Stars, mercenaires et journaliers sont exclus — pas de fiche `Player` durable côté `players`.
- `Passe` et `Lancer` (types d'action `match_report`) sont fusionnés en une seule notion domaine : `PassCompleted`.
- `round_label` et les noms d'équipe sont résolus par `match_report` via les ports ACL existants (`ICompetitionDataPort`, `ITeamDataPort`) **au moment de la publication**, injectés dans le publisher, puis embarqués tels quels dans chaque event — aucune dénormalisation sur l'agrégat `MatchReport`, aucun appel inter-BC nécessaire côté lecture pour `players`.

### Barème SPP

- Vit dans `references`, exposé via des méthodes de port parlantes par type d'action (`GetTouchdownSpp()`, etc.).
- `players` résout le SPP lui-même, via son propre port + un domain service, **avant** d'appeler la méthode domaine sur `Player` — `match_report` n'a aucune connaissance du SPP.
- Essai, Passe, Interception, Sortie, MVP rapportent du SPP. Agression et blessure n'en rapportent jamais.

### Statut de participation et blessures — voir `player-report-events/06-domaine.md`
