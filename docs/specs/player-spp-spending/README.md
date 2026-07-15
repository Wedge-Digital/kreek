# Player SPP spending — Spec index

Dépense des SPP (Star Player Points) d'un joueur sur des compétences
additionnelles ou des augmentations de caractéristiques, pendant la phase
`PlayerImprovement` de son équipe. Complète la carte 36 (to_be_refined),
en révisant son architecture d'origine : c'est le BC `players` qui possède
la commande et l'agrégat (pas `teams`), car tout ce qui existe déjà
(compétences acquises, matrice de coût BB2020, widget de sélection) y vit
déjà.

Une seule page concernée : la fiche joueur existante
(`src/app/players/io/web/templates/player-detail.html`), maquette de
référence `assets/rawpages/html/app-player-detail.html` (coûts affichés à
titre indicatif — remplacés par la vraie matrice `skill_cost.json`).

## Pages

| Page | Front | Back | DTOs | Use cases | Domaine | Intégration | Cartes |
|---|---|---|---|---|---|---|---|
| player-detail (slot droit) | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ (9 cartes) |

## Règles métier (validées)

1. Réserve de SPP = `player.spp` (gagné) − somme des coûts déjà dépensés (dérivé, pas de compteur stocké).
2. Niveau de progression = compteur **unique partagé** compétences + caractéristiques : nombre total d'améliorations déjà achetées + 1, plafonné à 6.
3. Coût d'une compétence : dépend du niveau, du mode (Choisie/Aléatoire déclaré par le coach), et de l'accès de sa catégorie (primary/secondary) par rapport à la position du joueur. Catégorie hors accès → achat impossible.
4. Statut élite d'une compétence majore le coût si une valeur elite est définie pour ce niveau ; sinon tarif standard (déjà géré par `chosen_for`/`random_for` côté `references`).
5. Coût d'une augmentation de caractéristique : dépend uniquement du niveau (`characteristic` de la matrice), identique quel que soit MA/ST/AG/PA/AV.
6. Compétence déjà possédée (base ou acquise) → achat impossible.
7. Achat immédiat et définitif — pas de panier, pas d'annulation.
8. Dépense possible uniquement si l'équipe est en phase `PlayerImprovement` — vérifié à l'affichage **et** à la mutation.
9. Autorisé : coach de l'équipe, admin de compétition, ou admin d'espace.
10. Coût toujours recalculé serveur via le port catalogue — jamais accepté du client.
11. Chaque achat produit un événement domaine immuable, rejouable via `hydrate()`.
12. `teams` est informé via un app event émis par `players`, consommé par un nouveau listener qui construit `TeamDomainEvent::PlayerImprovementApplied` (déjà défini, jamais construit).
13. `Player.value` (affiché sur la fiche) est aussi incrémenté à chaque achat, via `value_delta` porté par l'événement — table de référence (officielle, fournie par l'utilisateur, remplace la table provisoire de la carte 36) :

| Augmentation | Valeur |
|---|---|
| Compétence principale (primary) | +20 kPo |
| Compétence secondaire (secondary) | +40 kPo |
| +1 AV | +10 kPo |
| +1 MA | +20 kPo |
| +1 PA | +20 kPo |
| +1 AG | +30 kPo |
| +1 ST | +60 kPo |

Le mode (choisie/aléatoire) n'influence pas cette valeur.

## Architecture — écarts par rapport à la carte 36 originale

- **Propriétaire de la commande** : `players` (pas `teams`) — décision validée en phase 1, la carte 36 sera mise à jour/fermée en conséquence une fois cette feature livrée.
- **Pas de nouveau port ACL `players → teams`** pour la lecture (phase/permission) — réutilisation du précédent déjà établi dans `player_detail_controller.rs` (accès direct à `teams::Team`, comme `check_admin_rights` le fait déjà pour `can_customise`).
- **Nouveau port ACL `players → references`** (`ISkillCatalogPort`) — respecte la règle "Adapters inter-BCs", DTOs propres à `players`.
- **Patron front** : slot unique sur la fiche joueur (`pd-right-panel`), rempli par l'un de plusieurs widgets mutuellement exclusifs (journal par défaut, dépense SPP si éligible, customisation en future carte) — décision explicite de l'utilisateur, cf. discussion phase 2.
- **Achat immédiat, pas de panier** — décision explicite, s'écarte de la maquette `app-player-detail.html` qui montrait un panier différé.
