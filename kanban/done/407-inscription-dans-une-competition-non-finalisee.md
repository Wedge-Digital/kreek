# On peut inscrire une équipe dans une compétition non finalisée

> **⚠️ Défaut constaté en production, et silencieux.** L'équipe est créée,
> acceptée, facturée — et reste en attente d'inscription dans une compétition
> réglée en inscription automatique. Aucune erreur, aucune ligne de journal.

**Priorité : haute** — arrivé en production sur la Saison 10 le 2026-08-24
**Dépend de :** rien
**Trouvé le :** 2026-08-26, en cherchant un `TeamEnrolled` jamais émis

## Le symptôme

Sur la Saison 10, une équipe sur seize n'est jamais entrée dans la compétition.
La compétition est pourtant réglée en **inscription automatique**
(`requires_validation: false`), et les quinze autres équipes s'y sont inscrites
sans incident.

## La chaîne exacte

**1. Le brouillon de compétition est joignable dès que ses règles sont posées.**
`post_draft_team.rs:74` consulte `find_creation_rules_for_season`, dont
l'adapter (`competition_rules_adapter.rs:20`) ne demande que l'existence des
règles :

```rust
let full = self.season_repo.find_full(&sid).await.ok()??;
let rules = full.rules?;      // ← suffit : le statut n'est jamais regardé
```

Or le statut de saison suit `rules_selected` → `structure_selected` →
`invitations_configured` → `ready`. **Un coach peut donc bâtir son équipe dès
la première de ces quatre étapes.**

**2. `invitations` n'existe pas encore à ce moment.**
`create_draft_competition` ne l'écrit jamais ; la colonne est nullable et n'est
renseignée qu'à la phase 4, par `save_competition_invitations`.

**3. L'inscription automatique retombe alors sur « non ».**
`finalize_team.rs:107` :

```rust
.find_invitations(&sid).await
    .ok().flatten()
    .map(|inv| !inv.requires_validation.0)
    .unwrap_or(false)      // ← invitations absente ⇒ « pas automatique »
```

L'équipe reste en `PendingEnrollment`, `TeamEnrolled` n'est jamais émis, et rien
ne le signale.

## Ce que la production montre

| Heure | Événement |
|---|---|
| 14:24:39 | `CompetitionCreated` — brouillon, `invitations` NULL |
| **14:34:20** | `TeamCreated` « La Bande Des Saltimbanques Mediocres » — **sans `TeamEnrolled`** |
| 14:37:14 | `CompetitionReady` |
| 14:51:52 → 21:57 | 15 équipes, chacune avec son `TeamEnrolled` **dans la même seconde** |

La corrélation est parfaite : la seule équipe créée avant la finalisation est la
seule qui ne s'est pas inscrite.

## Le correctif

Refuser d'ouvrir une création d'équipe sur une saison dont le statut n'est pas
`ready`, à l'entrée (`post_draft_team.rs`) et à la soumission
(`submit_team.rs`, `finalize_team.rs`) — une équipe peut être commencée avant la
finalisation et soumise après, la garde d'entrée seule ne suffit pas.

**Le refus doit être explicite.** Faire simplement renvoyer `None` à l'adapter
rendrait « saison pas prête » indiscernable de « saison sans règles » — on
remplacerait un silence par un autre. Le port doit distinguer les deux cas, le
handler doit rendre un message que le coach comprend, et le refus doit laisser
une ligne de journal.

## Ce que cette carte ne couvre pas

La dégradation silencieuse d'`auto_enroll` elle-même — panne du dépôt ou
identifiant de saison illisible retombent toujours sur `false` sans un mot, et
le calcul est écrit trois fois à l'identique. Sixième défaut de cette famille ;
carte à part.

La remise en état des Saltimbanques : **déjà faite à la main.**

## Terminé quand

- Un coach ne peut ni ouvrir ni soumettre une équipe sur une saison non `ready`
- Une saison `ready` reste joignable — sans quoi la garde peut fermer la porte à
  tout le monde sans qu'aucun test ne le voie
- Le refus est journalisé et affiché
- Test unitaire sur la garde ; test e2e : compétition en brouillon ⇒ création
  refusée
