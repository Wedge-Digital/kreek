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

- [x] `widgets/notification_settings_widget.rs` : GET (fragment) + POST (`204`)
- [x] `templates/widgets/notification-settings-widget.html` :
      `hx-disinherit="*"`, Alpine `init()`/`destroy()`, `<link>` vers son CSS
- [x] `assets/static/css/widgets/notification-settings.css`
- [x] Deux routes ; un `mode` inconnu vaut `400`, pas un repli silencieux
- [x] `NotificationSettingsVm::from_domain()` — VM co-localisé, il ne dépend que
      du domaine
- [x] `deadline_cleared_reason` présent **même quand la ligne démarre
      applicable** : sinon le client n'a rien à afficher quand il en a besoin
- [x] Branchement dans l'onglet Synthèse, mode `autosave`
- [x] E2E : bascule sur compétition démarrée → persistée après rechargement
- [x] E2E : lignes de journée grisées avec leur motif sur une compétition sans
      calendrier
- [x] E2E : après la bascule, **la carte de la compétition mène toujours au
      détail** — non-régression du statut
- [x] `tests/impact-map.toml` mis à jour dans le même commit
- [x] `make check-arch` et `make e2e`

## Ce qui a été fait, et ce qui s'en écarte

**Pas de `<link>` dans le fragment**, contrairement à ce que demandaient la
checklist et la phase 7. La carte 342 a supprimé ce mécanisme : plus aucun
fragment ne porte de feuille, et l'axe 14 de `check-arch` refuse une feuille
absente du bundle. `widgets/notification-settings.css` est donc déclarée dans
`css_bundle.rs`, et scopée sous `.notification-settings` comme l'impose la 341.
L'intention de la checklist — CSS embarqué, aucune dépendance au layout de
l'hôte — est tenue par le moyen d'aujourd'hui.

**Les quatre newtypes ont été renommés** en `NotifyRegistrationOpen`,
`NotifyRoundEve`, `NotifyRoundClosing`, `NotifyRegistrationDeadline` : la
phase 4 les nommait ainsi, la 331 les avait écrits sans le préfixe.

## Quatre pièges, tous invisibles au test unitaire

Le widget se servait en 200 par `curl` et restait absent de la page.

1. **Les routes n'étaient pas branchées** — 404. Le `use` était en place, le
   `.route()` non.
2. **`/admin/summary` rend un fragment**, sans layout donc sans HTMX : le
   `hx-trigger="load"` du conteneur n'y part jamais. Le test doit passer par la
   page d'admin et cliquer l'onglet.
3. **`hx-include` manquait.** HTMX ne sérialise les champs d'un élément que si
   c'est un formulaire ou un champ : depuis un `div`, le corps part **vide**.
   Et `find .ns-check` ne renvoie que le **premier** descendant — une case sur
   quatre.
4. **Une case cochée envoie `on`, pas `true`** : le POST rendait **422**. Résolu
   par un `deserialize_with` faisant de la seule présence une vérité, plutôt que
   par un `value="true"` dans le gabarit — un DTO de transport doit lire ce
   qu'un formulaire envoie réellement, pas ce qu'un attribut lui promet.

Les trois derniers ont le même symptôme : les cases affichent une chose, la base
en enregistre une autre. La carte annonçait ce piège ; elle n'en nommait qu'une
des trois formes.

## Découvert au passage, non corrigé

`scripts/check-css-collisions.sh` est **rouge et branché nulle part** — ni
`make lint`, ni `make check-arch`, ni la CI. Son contrôle A passe (46/46
feuilles conformes, celle de cette carte comprise) ; son contrôle B échoue sur
six sélecteurs, dont quatre `.ts-*` venus de `vendor/tom-select.min.css`, entrée
dans le bundle à la carte 17. Carte séparée.
