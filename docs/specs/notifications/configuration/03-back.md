# Phase 3 — Architecture back : l'écran de réglage des notifications

**Entrée** : `02-front.md`, validée.

## La trouvaille qui gouverne le stockage

**Réutiliser l'enregistrement des invitations ferait reculer une compétition en
cours.** Les quatre `UPDATE` du magicien écrivent tous `status` au passage :

```sql
-- update_invitations.sql
UPDATE competition_seasons
SET    invitations = $1::jsonb,
       status      = 'invitations_configured'
WHERE  id          = $2
```

Ce statut n'est pas décoratif. Dans `all_competition.rs`, il décide **où mène le
clic sur la carte de la compétition** :

| `status` | destination de la carte |
|---|---|
| `draft` | étape 1 du magicien |
| `rules_selected` | étape 2 |
| `structure_selected` | étape 3 |
| `invitations_configured` | étape 5 (validation) |
| autre (`ready`…) | le détail de la compétition |

Un organisateur décochant une notification depuis l'admin verrait donc sa
compétition vivante retomber à « en cours de création », et sa carte le renvoyer
dans le magicien. L'auto-save décidé en phase 2 marchait droit dessus.

## Stockage : une quatrième colonne JSONB `notifications`

`competition_seasons` porte déjà `rules`, `structure`, `invitations` — une par
étape du magicien. Une quatrième colonne s'ajoute, avec son propre couple
select/update **qui ne touche pas `status`**.

Trois arguments, d'inégale force, dans cet ordre :

**Le plus fort — l'écriture concurrente.** Loger les quatre booléens dans le blob
`invitations`, c'est lire-modifier-réécrire un objet qui contient aussi
`invited_coaches`. Une bascule en admin pendant qu'un autre onglet invite un
coach écrase l'invitation. Une colonne à part réduit l'écriture à
`SET notifications = $1` et ne touche rien d'autre.

**La cohérence — aucune colonne existante n'est leur maison.** Deux des quatre
réglages concernent les journées, configurées dans `structure` ; deux concernent
l'inscription, dans `invitations`. Les répartir serait pire que les regrouper, et
ce qu'ils ont en commun — la communication — n'est ni la forme de la compétition
ni qui a le droit d'y entrer.

**Le plus faible — le statut.** Il n'impose pas une colonne, seulement un
`UPDATE` distinct : on pourrait écrire le JSONB des invitations sans toucher au
statut. Il force une requête nouvelle, pas un stockage nouveau. Dit franchement
pour qu'on ne croie pas la colonne mieux justifiée qu'elle ne l'est.

## Plan de fichiers

```
migrations/
└── <ts>_competition_season_notifications.sql      ← ALTER TABLE, colonne JSONB

src/app/competitions/
├── domain/
│   ├── competition_notifications.rs               ← les 4 VOs, la struct, l'applicabilité
│   └── season_repository_port.rs                  ← + find_notifications / save_notifications
├── use_cases/
│   └── save_competition_notifications.rs          ← commande + execute
├── io/
│   ├── repository/sql/seasons/
│   │   ├── select_notifications.sql
│   │   └── update_notifications.sql               ← sans status
│   └── web/
│       ├── widgets/
│       │   └── notification_settings_widget.rs    ← GET (fragment) + POST (auto-save)
│       └── templates/widgets/
│           └── notification-settings-widget.html
└── routes.rs                                      ← 2 routes

assets/static/css/widgets/
└── notification-settings.css                      ← CSS embarqué (règle 5 des widgets)
```

Fichiers **modifiés** : `new-competition-phase-4.html` (le widget remplace la
case `notify_by_email`, et la section 4 émet `registrationDeadlineChanged`),
`new-competition-phase-3.html` (retrait de l'interrupteur `use_mail_notification`),
et le template de l'onglet Synthèse.

## Routes

```
GET  /app/{space_id}/competitions/{competition_id}/{season_id}/notifications-widget?mode=
POST /app/{space_id}/competitions/{competition_id}/{season_id}/notifications
```

Les deux portent `{season_id}`, et **`SeasonSpaceOwnership` existe déjà** et est
enregistré dans `main.rs` : le cloisonnement par espace s'applique sans rien
ajouter. Une route de réglage atteignable depuis un espace étranger aurait été
exactement le défaut de la carte 316.

## Ports et domain services

**Aucun port, aucun adapter.** `competitions` possède les saisons ; la donnée ne
franchit aucune frontière de BC. La règle des adapters inter-BCs ne s'applique
pas à cet écran.

**Une fonction de domaine, pas une logique de handler.** Le calcul
d'applicabilité (R5) répond à « cette notification a-t-elle un sens ici ? » —
une question métier, selon la grille de décision du CLAUDE.md. Elle vit dans
`competition_notifications.rs`, prend la `CompetitionStructure` et rend
l'applicabilité des quatre réglages. Le handler la consulte, ne la refait pas.

Rappel de la répartition établie en phase 2 : deux motifs se calculent au GET
(pas de calendrier, aucune journée `time_frame`), le troisième vit côté client
puisque la date limite se saisit dans le même écran.

## Ce que cet écran ne fait pas

**Il ne résout aucun destinataire.** La configuration dit *quelles* notifications
partent, jamais *à qui* — c'est le sujet de R7, traité dans `envoi/`. Cette
séparation est ce qui permet à `configuration/` de ne dépendre d'aucun port.

## Règles métier

Aucune règle nouvelle n'apparaît à cette phase. R6 (phase 2) et R5 (phase 1)
sont celles qui contraignent le plan ci-dessus ; R7, apparue pendant cette phase,
relève de `envoi/` et est consignée dans le README.

## Ce que cette phase laisse aux suivantes

- **Phase 4** — les noms des quatre champs, et si les motifs d'inapplicabilité
  voyagent dans le DTO du GET ou sont recalculés au rendu.
- **Phase 7** — la migration des deux interrupteurs morts vers la nouvelle
  colonne, pour les ~399 saisons existantes, et le défaut appliqué à celles qui
  n'ont jamais rien coché.
