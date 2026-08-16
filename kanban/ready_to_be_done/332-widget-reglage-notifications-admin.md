# Widget de réglage des notifications — et son hôte admin

**Spec :** `docs/specs/notifications/configuration/` (phases 2, 3, 7)
**Dépend de :** 331

> **Ne pas livrer seule.** Cette carte fait partie d'une chaîne de dix. Aucune
> carte avant la 340 ne fait partir un email : livrer `configuration/` sans
> `envoi/` produirait un **troisième interrupteur email mort**, mieux dessiné
> que les deux qu'il remplace et tout aussi inerte — le défaut même que cette
> fonctionnalité corrige. Cf. `docs/specs/notifications/README.md`.

## Objectif

Le widget de réglage, et son premier hôte : l'onglet Synthèse de la page
d'administration, en mode auto-save. Première carte livrant quelque chose
d'utilisable — un organisateur peut régler les notifications d'une compétition
déjà démarrée.

## Conception

### Pourquoi un widget et pas une section

**Aucun chemin ne permettait de modifier ces réglages après la création** :
`save_competition_invitations` n'a qu'un appelant, le POST du magicien, et la
Synthèse les affiche en lecture seule. Sans chemin d'édition, les ~399 saisons
existantes resteraient figées sans recours.

### Le piège du POST : une case décochée n'est pas envoyée

`hx-post` poste un **formulaire**. Une case non cochée n'apparaît pas dans le
corps de la requête.

```rust
#[derive(Debug, Deserialize)]
pub struct NotificationSettingsPayload {
    #[serde(default)] pub registration_open: bool,
    #[serde(default)] pub round_eve: bool,
    #[serde(default)] pub round_closing: bool,
    #[serde(default)] pub registration_deadline: bool,
}
```

Extracteur **`Form`, pas `Json`**. Le symptôme d'une erreur ici serait trompeur :
on pourrait activer une notification et jamais la désactiver, ce qui ressemble à
un défaut de persistance alors que le corps est incomplet.

### L'événement dit « voici l'état », pas « ça a bougé »

Le widget émet `notificationSettingsChanged` à son `init()` **et** à chaque
bascule. C'est ce qui permettra à la carte 333 de réhydrater la page hôte sans
aller lire dans le DOM du widget.

## Checklist

- [ ] `widgets/notification_settings_widget.rs` : GET (fragment) + POST (`204`)
- [ ] `templates/widgets/notification-settings-widget.html` :
      `hx-disinherit="*"`, Alpine `init()`/`destroy()`, `<link>` vers son CSS
- [ ] `assets/static/css/widgets/notification-settings.css`
- [ ] Deux routes ; un `mode` inconnu vaut `400`, pas un repli silencieux
- [ ] `NotificationSettingsVm::from_domain()` — VM co-localisé, il ne dépend que
      du domaine
- [ ] `deadline_cleared_reason` présent **même quand la ligne démarre
      applicable** : sinon le client n'a rien à afficher quand il en a besoin
- [ ] Branchement dans l'onglet Synthèse, mode `autosave`
- [ ] E2E : bascule sur compétition démarrée → persistée après rechargement
- [ ] E2E : lignes de journée grisées avec leur motif sur une compétition sans
      calendrier
- [ ] E2E : après la bascule, **la carte de la compétition mène toujours au
      détail** — non-régression du statut
- [ ] `tests/impact-map.toml` mis à jour dans le même commit
- [ ] `make check-arch` et `make e2e`
