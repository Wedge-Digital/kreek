# Administration d'espace — Progression

## Maquette (Phase 1 ✅)

`assets/rawpages/html/app-space-admin.html` — une page, quatre onglets. Validée
et commitée (`cfe938e`).

## Progression par onglet

| Onglet | Front | Back | DTOs | Use cases | Domaine | Intégration | Cartes |
|---|---|---|---|---|---|---|---|
| Page hôte + Membres | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ (364-374) |
| Ajout direct | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ (376-384) |
| Invitations | | | | | | | |
| Paramètres | | | | | | | |

La page hôte est spécifiée **avec** l'onglet Membres : elle est le cadre que les
trois autres remplissent, et elle n'a pas de contenu propre au-delà de la
bannière et de la barre d'onglets.

## Périmètre

### Ce que la fonctionnalité couvre

Les quatre onglets de la maquette, **moins la zone de danger**.

### Ce qu'elle ne couvre pas — épic dédiée

**La zone de danger** — transférer la propriété, archiver, supprimer l'espace.
Sortie du périmètre à la phase 1, pour deux raisons qui n'ont rien à voir avec
sa taille :

- elle touche au **cycle de vie de l'espace**, pas à ses membres ni à ses
  réglages ; c'est un autre sujet sous le même onglet ;
- supprimer un espace veut dire supprimer ses équipes et ses compétitions,
  c'est-à-dire commander la destruction de données dont d'autres BCs sont
  souverains. La règle de souveraineté l'interdit frontalement, et contourner
  cet interdit est une conception à part entière.

Aucune des trois opérations n'a d'ailleurs d'objet auquel s'appliquer
aujourd'hui : ni propriétaire, ni état d'archivage n'existent en base.

## Ce qui n'existe pas et que la fonctionnalité doit construire

Constaté à la phase 1, sur le code et le schéma :

| Manque | Constat |
|---|---|
| **Invitations d'espace** | ni table, ni use case, ni route. Le seul `invitations` du dépôt est celui des compétitions |
| **Visibilité** | la table `spaces` porte `id`, `space_name`, `space_icon_path`, `created_at`, `legacy_id`. Rien pour public/privé |
| **Changement de rôle** | `spaces__user_space.profile` porte la valeur, aucun use case ne la modifie après coup |
| **Retrait d'un membre** | aucune opération |

Trois des cinq événements du domaine `spaces` sont **définis et jamais émis** :
`UserInvitedInSpace`, `SpaceArchived`, `UserPromotedToSpaceAdmin`. Les deux
premiers relèvent respectivement de l'onglet Invitations et de l'épic zone de
danger ; le troisième est à reprendre par l'onglet Membres.

## Ce sur quoi on s'appuie

- **`SpacePermissions`** — extracteur déjà écrit, avec `is_admin()`. La porte de
  la page existe.
- **`ISpacesHostLayout::upload_widget()`** — le logo de l'onglet Paramètres.
- **`auth::send_reset_password_email`** et `IEmailService`, deux
  implémentations branchées.
- **`spaces__user_cache`**, alimenté par `user_created_listener`, contient
  **tous** les coachs de la plateforme. L'annuaire nécessaire à l'Ajout direct
  est donc déjà là, sans franchir de frontière de BC.
- **L'annuaire d'espaces** — `/app/space/all`, `find_all()`. C'est lui qu'un
  espace privé doit déserter.

Attention : le widget `space-members-widget` **ne se réutilise pas**. Malgré son
nom, c'est un sélecteur de coachs pour formulaires, pas une liste
d'administration.

## Règles métier validées en phase 1

1. **Un espace a toujours au moins un administrateur.** Le dernier ne peut être
   ni rétrogradé, ni retiré, par personne — lui compris.
2. **On ne modifie pas son propre rôle et on ne se retire pas soi-même.**
3. **Retirer un coach est autorisé même s'il a une équipe engagée** en
   compétition. Cela ne change ni le déroulement de la compétition, ni la
   capacité à saisir des matchs.
4. **La visibilité gouverne l'annuaire** : un espace privé n'apparaît pas dans
   `/app/space/all`. L'annuaire existe, il est à filtrer.
5. **L'invitation nominative se fait par recherche, jamais par saisie libre** :
   on ne peut pas inviter un coach qui n'existe pas. Le cas « ce coach n'a pas
   de compte » est traité par l'Ajout direct, qui crée le compte.
6. **La réinitialisation de mot de passe envoie un email**, y compris à
   soi-même. La maquette laissait entendre qu'on copiait un lien pour soi et
   qu'on envoyait un mail aux autres : ce n'est pas retenu, le comportement est
   le même pour tous.
7. **Seul un administrateur d'espace accède à la page** — `is_admin()`.

## Un défaut à corriger en chemin

`spaces/domain/domain_event.rs` : `USER_SUBSCRIBED_TO_SPACE` et
`USER_INVITED_IN_SPACE` valent **la même chaîne**, `"UserRegisteredInSpace"`.
Deux événements distincts partagent leur type, donc tout listener qui filtre
dessus attrape les deux.

Latent aujourd'hui, puisque `UserInvitedInSpace` n'est jamais émis. L'onglet
Invitations va précisément l'émettre : **le défaut doit être corrigé avant**,
pas en même temps.

## Structure des specs

```
docs/specs/space-admin/
├── README.md
├── membres/          ← page hôte + onglet Membres
│   ├── 02-front.md   ✅
│   ├── 03-back.md    ✅
│   ├── 04-dtos.md    ✅
│   ├── 05-use-cases.md ✅
│   ├── 06-domaine.md  ✅
│   ├── 07-integration.md ✅
│   └── 08-cards.md   ✅ → cartes 364-374
├── ajout-direct/
│   ├── 02-front.md   ✅
│   ├── 03-back.md    ✅
│   ├── 04-dtos.md    ✅
│   ├── 05-use-cases.md ✅
│   ├── 06-domaine.md  ✅
│   ├── 07-integration.md ✅
│   └── 08-cards.md   ✅ → cartes 376-384
├── invitations/
└── parametres/
```
