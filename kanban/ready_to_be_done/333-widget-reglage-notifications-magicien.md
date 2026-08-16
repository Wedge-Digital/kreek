# Widget de réglage des notifications — dans le magicien

**Spec :** `docs/specs/notifications/configuration/` (phases 2, 4, 7)
**Dépend de :** 331, 332

> **Ne pas livrer seule.** Cette carte fait partie d'une chaîne de dix. Aucune
> carte avant la 340 ne fait partir un email : livrer `configuration/` sans
> `envoi/` produirait un **troisième interrupteur email mort**, mieux dessiné
> que les deux qu'il remplace et tout aussi inerte — le défaut même que cette
> fonctionnalité corrige. Cf. `docs/specs/notifications/README.md`.

## Objectif

Installer le widget à l'étape 4 du magicien, en mode différé, et retirer la case
`notify_by_email` qu'il remplace.

## Conception

### Mode différé, et non auto-save

L'étape 4 enregistre **d'un bloc**. Si les notifications s'y sauvaient seules, un
« ← Retour » laisserait les cases persistées et la date limite perdue : deux
comportements de sauvegarde dans un même écran, sans rien pour les distinguer.

### La réhydratation de l'hôte — le piège de cette carte

Le widget est rendu par le serveur, donc les cases reviennent correctes. Mais
l'objet `state` de la page, lui, doit être réhydraté **aussi** :

```js
state.notifications = INITIAL_NOTIFICATIONS;   // depuis existing_notifications_json
```

Sans cela : retour arrière, re-validation sans toucher aux cases → le défaut de
la page part au serveur et **écrase les réglages sauvegardés**, pendant que
l'écran affiche autre chose. Le code actuel fait déjà la bonne chose pour
`notify_by_email` ; déplacer la case dans un widget déplace son rendu, pas la
réhydratation.

Deux mécanismes, deux rôles : le JSON rendu par le serveur garantit que `state`
est juste **dès la première peinture**, sans dépendre du `hx-get` du widget ;
l'émission à l'`init()` (carte 332) resynchronise ensuite.

### Le grisage vivant de la quatrième ligne

La section 4 émet `registrationDeadlineChanged` à la frappe ; le widget écoute et
grise. Seul endroit où le magicien parle au widget, et il passe par `body`.

## Checklist

- [ ] Conteneur du widget en mode `deferred` à l'étape 4, à la place de la case
      `notify_by_email`
- [ ] `existing_notifications_json` rendu par le handler GET de l'étape 4
- [ ] `state.notifications` réhydraté depuis ce JSON
- [ ] La page hôte écoute `notificationSettingsChanged` et fusionne dans `state`
- [ ] La section 4 émet `registrationDeadlineChanged` à la frappe
- [ ] `notify_by_email` retiré du corps POST, du template et de
      `CompetitionInvitations`
- [ ] `save_invitations` change de signature (un seul appelant) ;
      `update_invitations.sql` écrit les deux colonnes et **garde** son `status`
- [ ] E2E : saison neuve → quatre cases cochées
- [ ] E2E : décocher, continuer, revenir → l'état est affiché
- [ ] **E2E de réhydratation** : décocher, continuer, revenir, **re-valider sans
      toucher aux cases**, revenir → l'état a tenu
- [ ] E2E : effacer la date limite → quatrième ligne grisée **sans rechargement**
- [ ] E2E : cocher la date limite puis effacer la date → **reste cochée** (R6)
- [ ] `tests/impact-map.toml` mis à jour dans le même commit
- [ ] `make check-arch` et `make e2e`
