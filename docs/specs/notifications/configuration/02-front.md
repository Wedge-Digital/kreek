# Phase 2 — Architecture front : l'écran de réglage des notifications

**Entrée** : `assets/rawpages/html/competition-notifications-config.html`, validée
en phase 1.

## Ce que l'existant impose, et qui n'était pas su en phase 1

### Il y a deux interrupteurs email morts, pas un

La phase 1 avait relevé `notify_by_email` (étape 4 du magicien). Il en existe un
**second**, à l'étape 3 — `use_mail_notification`, « Notifications e-mail :
Activées / Désactivées », sous cette note :

> 📌 Des rappels seront envoyés aux équipes à l'ouverture et à la fermeture de la
> phase, ou pour confirmer la participation au multiplexe.

C'est mot pour mot les notifications 2 et 3. Stocké en JSONB, lu par personne :
aucune lecture hors de la struct de domaine. Le magicien promettait donc déjà
cette fonctionnalité **à deux endroits**, et ne la rendait nulle part.

**Les deux disparaissent** au profit des quatre réglages, qui les absorbent.
Laisser trois endroits parler d'email dont deux sans effet aurait rendu l'écran
moins lisible après qu'avant.

### Rien ne permettait de modifier ces réglages après la création

`save_competition_invitations` a exactement un appelant : le POST de l'étape 4.
L'onglet Synthèse les affiche en lecture seule, sans lien d'édition. Ce qui est
coché au magicien était donc figé — et les ~399 saisons existantes seraient
restées sur les valeurs par défaut, sans recours.

**Décision : les réglages sont éditables aux deux endroits**, magicien et admin.
C'est ce qui fait du bloc un widget plutôt qu'une section de plus.

### Les motifs d'inapplicabilité (R5) ont trois sources, dont une vivante

| Notification | Inapplicable si | Source | Calcul |
|---|---|---|---|
| Veille de journée | pas de calendrier (`use_schedule = false`) | étape 3 | serveur, au GET |
| Fin de journée imminente | pas de calendrier, ou aucune journée `time_frame` | étape 3 | serveur, au GET |
| Date limite d'inscription | `registration_deadline` vide | **étape 4, même écran** | **client, à la frappe** |

Une journée `fixed_date` n'a pas de fenêtre à clore : seule une journée
`time_frame` porte une `end_date`. C'est pourquoi la troisième notification
demande plus que l'existence d'un calendrier.

La troisième ligne est la seule contrainte réelle de cette phase : le champ dont
elle dépend est **dans le même écran**, et son grisage doit suivre la frappe.
Un aller-retour serveur à chaque caractère serait absurde.

## Le widget

| Widget | BC | Endpoint | Trigger | Émet | Mode |
|---|---|---|---|---|---|
| `notification-settings` | competitions | `GET …/{season_id}/notifications-widget?mode=` | `load` | `notificationSettingsChanged` | différé / auto-save |

Le BC `competitions` possède les saisons : **ni port ni adapter** à créer, la
règle des adapters inter-BCs ne s'applique pas ici.

Aucun widget existant n'est réutilisable — les quarante widgets du projet ne
comptent aucun composant de réglages.

## Événements

- **`notificationSettingsChanged`** : `{ registration_open, round_eve,
  round_closing, deadline }` — quatre booléens. Émis par le widget **à son
  `init()` puis à chaque bascule**. Écouté par la page du magicien, qui le
  fusionne dans son objet `state` existant ; et, en mode auto-save, **par le
  widget lui-même**, dont la racine porte `hx-post` sur
  `hx-trigger="notificationSettingsChanged from:body"`.

  Les noms de champs sont provisoires — la phase 4 les fixe.

  **L'émission à l'`init()` n'est pas une précaution : elle change ce que
  l'événement veut dire.** « Quelque chose a bougé » n'est pas consommable par un
  hôte qui doit connaître l'état même quand rien n'a bougé ; « voici l'état
  courant » l'est. Sans elle, revenir sur l'étape 4 et re-valider sans toucher
  une case écraserait les réglages sauvegardés par le défaut de la page — les
  cases affichant une chose pendant qu'une autre part au serveur.

- **`registrationDeadlineChanged`** : `{ value: string|null }` — émis par la
  section 4 de l'étape 4 à la frappe, écouté par le widget pour griser ou
  dégriser la quatrième ligne.

  C'est le seul endroit où le magicien parle au widget, et il passe par `body` :
  aucun appel direct, conformément à la règle 2 des conventions widgets.

## Deux modes, et pourquoi pas de l'auto-save partout

Le magicien enregistre l'étape 4 **d'un bloc**, au bouton « Enregistrer &
continuer ». Si les notifications se sauvaient seules, un « ← Retour » laisserait
les cases persistées et la date limite perdue : deux comportements de
sauvegarde dans un même écran, sans rien pour les distinguer. Le mode différé
aligne le widget sur son hôte.

**L'événement reste le contrat unique.** Le widget émet toujours, quel que soit
le mode ; seule la persistance change, et elle tient dans deux attributs HTMX
posés au rendu. Aucune branche JS entre les deux modes — c'est ce qui rend les
deux modes acceptables plutôt qu'un doublon déguisé.

| Mode | Hôte | Ce que le widget fait de son propre événement |
|---|---|---|
| `deferred` | étape 4 du magicien | rien — la page hôte l'écoute et POSTe plus tard |
| `autosave` | onglet Synthèse de l'admin | `hx-post` sur sa propre racine |

## Front / back

**Front** — la bascule des cases, le grisage vivant de la quatrième ligne,
l'émission de l'événement. Alpine, avec `init()`/`destroy()` comme l'exige la
règle 7 des conventions widgets.

**Back** — l'état initial et les deux motifs structurels, calculés au GET depuis
la structure de la saison ; la persistance au POST, en mode auto-save seulement.

## Actions

```
GET  …/{season_id}/notifications-widget?mode=deferred|autosave
     → le fragment du widget, cases et motifs déjà résolus côté serveur

POST …/{season_id}/notifications          (mode auto-save uniquement)
     → 204, aucun swap : l'état visible est déjà celui de l'utilisateur
```

Un `204` et non un fragment : re-rendre le widget après chaque clic ferait
clignoter les cases et perdrait le focus clavier, pour réafficher exactement ce
qui est à l'écran.

## L'hôte admin : l'onglet Synthèse

Les réglages d'invitation y sont **déjà affichés** en lecture seule ; le bloc de
notifications s'y ajoute naturellement. Un onglet entier pour quatre cases serait
disproportionné.

À dire franchement : cela rend éditable un onglet qui ne l'était pas. C'est
assumé — le coût d'un onglet supplémentaire est plus élevé que celui de cette
entorse.

## Règle métier apparue à cette phase

### R6 — une notification cochée puis rendue inapplicable reste cochée

L'organisateur coche « date limite d'inscription », puis efface la date. **La
case reste cochée et grisée, la valeur stockée intacte.**

Décocher détruirait un choix explicite de l'organisateur en réaction à un geste
qui n'a rien à voir, et il ne s'en apercevrait pas. Le grisage dit déjà « sans
effet aujourd'hui » ; l'intention est conservée pour le jour où une date
reviendra.

C'est la même préférence que R1 : ne rien faire silencieusement plutôt que d'agir
à côté de ce qui a été demandé.

## Ce que cette phase laisse aux suivantes

- **Phase 3** — où vivent les quatre réglages : dans `CompetitionInvitations`
  aux côtés de `notify_by_email` qu'ils remplacent, ou dans une struct à eux ?
  Deux des quatre concernent les journées, dont la configuration est dans la
  structure, pas dans les invitations.
- **Phase 4** — les noms définitifs des quatre champs, et la forme du DTO du GET
  (les motifs d'inapplicabilité en font-ils partie, ou sont-ils recalculés ?).
- **Phase 7** — la migration des deux interrupteurs morts vers les nouveaux
  réglages, pour les ~399 saisons existantes.
