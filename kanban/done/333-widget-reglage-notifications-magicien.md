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

- [x] Conteneur du widget en mode `deferred` à l'étape 4, à la place de la case
      `notify_by_email`
- [x] `existing_notifications_json` rendu par le handler GET de l'étape 4
- [x] `state.notifications` réhydraté depuis ce JSON
- [x] La page hôte écoute `notificationSettingsChanged` et fusionne dans `state`
- [x] La section 4 émet `registrationDeadlineChanged` à la frappe
- [x] `notify_by_email` retiré du corps POST, du template et de
      `CompetitionInvitations`
- [x] `save_invitations` change de signature (un seul appelant) ;
      `update_invitations.sql` écrit les deux colonnes et **garde** son `status`
- [x] E2E : saison neuve → quatre cases cochées
- [x] E2E : décocher, continuer, revenir → l'état est affiché
- [x] **E2E de réhydratation** : décocher, continuer, revenir, **re-valider sans
      toucher aux cases**, revenir → l'état a tenu
- [x] E2E : effacer la date limite → quatrième ligne grisée **sans rechargement**
- [x] E2E : cocher la date limite puis effacer la date → **reste cochée** (R6)
- [x] `tests/impact-map.toml` mis à jour dans le même commit
- [x] `make check-arch` et `make e2e`

## Ce qui a été fait

`notify_by_email` disparaît du domaine, du corps POST et du gabarit — il n'avait
que cinq points d'usage, tous dans `new-competition-phase-4.html`. Retirer un
champ d'une struct serde ne casse pas la lecture des blobs déjà écrits, les clés
inconnues étant ignorées : **aucune réécriture des saisons existantes**.

`update_invitations.sql` écrit les deux colonnes **en une instruction**, et garde
son `status` — ici on est bien dans une étape du magicien, et c'est elle qui fait
avancer la saison. Deux `UPDATE` laisseraient une fenêtre où l'un a réussi et
l'autre non.

Le corps de l'étape 4 devient un DTO de transport : `#[serde(flatten)]` conserve
la forme historique et y ajoute le sous-objet `notifications`.

## Le piège de la carte s'est retourné contre son test

La carte met en gras un scénario de réhydratation : décocher, continuer,
revenir, **re-valider sans toucher aux cases**, revenir. Écrit tel quel, ce test
**passe alors même que `INITIAL_NOTIFICATIONS` est retiré** — vérifié en le
retirant. L'évènement émis à l'`init()` du widget corrige `state` avant qu'on
valide, donc le test n'exerce que ce chemin-là.

Les deux mécanismes ont des rôles distincts, la phase 4 de la spec le dit, et il
faut **deux tests** pour les couvrir :

| Test | Ce qu'il exerce | Tombe quand on retire |
|---|---|---|
| `…revalider_sans_toucher_aux_cases…` | l'évènement d'`init()` | l'écoute sur `body` |
| `…valider_avant_l_arrivee_du_widget…` | le JSON du serveur | `INITIAL_NOTIFICATIONS` |

Le second coupe le fragment du widget par `page.route(... abort)` : sans widget,
rien n'émet, et `state` ne peut être juste que si le serveur l'a renseigné dès
la première peinture. C'est le seul montage qui isole ce chemin.

Vérifié dans les deux sens, en retirant chaque mécanisme tour à tour. Sans le
JSON, un seul test tombe. Sans l'écoute, deux tombent — elle est aussi le seul
chemin par lequel une case cochée par l'utilisateur atteint `state`.

Le garde-fou global « aucune erreur de console » est désamorcé **localement**
dans ce test : couper le fragment fait crier HTMX, et tolérer ces erreurs
globalement masquerait de vraies pannes ailleurs.

## Détail relevé au passage

Le retour à l'étape 4 se fait par le bouton « ← Retour » de l'étape 5, jamais
par `page.go_back()` : le magicien navigue en `htmx.ajax` + `pushState` posés à
la main, et le retour navigateur ne re-rend pas la page.
