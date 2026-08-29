# Panneau « Tiers & coups de pouce »

**Épic :** E14 · **Ordre :** 3 · **Dépend de :** 417, 420
**Conception :** `docs/specs/modifier-une-competition/onglet-parametres/`
(`04-dtos.md`, `05-use-cases.md`)

## Objectif

Modifier les coups de pouce et les star players autorisés par tier, sur une
saison en cours. **Rien d'autre** : ni le nom, ni le budget, ni l'XP de départ,
ni les rosters.

## Le point le plus facile à rater

**Le widget de sélection n'a pas de champ caché.** `inducement-picker.html:15`
garde sa sélection dans son état Alpine et n'émet qu'un événement :

```js
htmx.trigger(document.body, 'inducementPickerChanged',
             { instanceId: this.instanceId, selected: this.selected });
```

Le panneau doit donc tenir une carte `instanceId → selected[]`, un `instanceId`
par tier, et la matérialiser au moment du POST. Sans ce JS de collecte, le
panneau enverrait des tiers aux listes vides **sans qu'aucune erreur ne le
signale** — les tiers perdraient tous leurs coups de pouce en silence.

C'est la seule communication par événement de cet onglet, et elle est interne au
widget des tiers. Elle existe déjà, on ne l'invente pas.

## Conception

### Le use case

```rust
pub struct UpdateTiersSettingsCommand {
    pub season_id: SeasonId,
    pub tiers: Vec<TierRule>,
}
```

1. `find_base_info` → le nom de saison
2. `find_rules` → `SeasonNotFound`, **le barème relu**
3. `current.with_inducements_from(cmd.tiers)?` — **le contrôle est dans le
   domaine** (carte 417)
4. `save_rules(…)`

Le use case appelle la méthode domaine et convertit son erreur ; il ne calcule
rien.

### Le contrôle des champs non éditables

`TierRule` est un tout et transporte le nom, le budget, l'XP et les rosters même
si le panneau ne les édite pas. `with_inducements_from` refuse tout écart, et
refuse un nombre de tiers différent.

**Un refus, pas une correction** : accepter la valeur reçue rendrait modifiable
par requête forgée ce que l'écran n'ouvre pas, et corriger en silence ferait
croire à un enregistrement qui n'a pas eu lieu.

**Un tier sans aucun coup de pouce est valide** — `Vec` vide accepté, aucune
borne basse.

### Le handler

```rust
GET  …/settings/tiers  → get_settings_tiers
POST …/settings/tiers  → post_settings_tiers   (Json)
```

```rust
#[derive(Deserialize)]
pub struct TiersSettingsPayload { pub tiers: Vec<TierRule> }
```

JSON parce que la cible est un agrégat imbriqué que les nutypes valident à la
désérialisation.

### Le VM

```rust
pub struct TierVm {
    pub index: u8,                 // la teinte : .tier-block--1, --2, …
    pub name: String,
    pub budget_kpo: u32,           // affichage seul
    pub starting_xp: u32,          // affichage seul
    pub roster_names: Vec<String>, // affichage seul
    pub inducements: Vec<ChipVm>,
    pub star_players: Vec<ChipVm>,
    pub picker_instance_id: String,
}
pub struct ChipVm { pub uid: String, pub label: String }
```

`ChipVm` et `roster_names` résolvent des uid par `ICompetitionReferencePort` →
`builders.rs`. **Un uid non résolu s'affiche tel quel** plutôt que de
disparaître : un coup de pouce retiré du corpus doit se voir, pas s'évaporer.

### Le template

Blocs de tier repris de `pages/new-competition-phase-2.css` — en-tête teinté,
badge coloré, corps blanc. Puces reprises de `widgets/inducement-grid.css`.
**Rien n'est réécrit**, rien n'est ajouté au bundle.

L'URL du picker vient de `app_routes.references.inducement_picker()`, comme
`new-competition-phase-2.html:154`.

## Tests

- Unitaires : couverts par la carte 417 pour le domaine ; ici, la relecture du
  barème et la conversion d'erreur.
- E2E : **modifier les coups de pouce d'un tier et vérifier qu'ils sont
  enregistrés** — c'est le seul test qui prouve que la collecte JS fonctionne,
  aucun test unitaire ne peut le voir.

## Checklist

- [x] Le use case et ses tests
- [x] Les deux handlers, `require_admin_access`
- [x] Le VM, `builders.rs`
- [x] Le template, le JS de collecte, les deux CSS repris
- [x] `make lint && make test && make check-arch`

## Ce que la réalisation a appris

### L'onglet porte les quatre panneaux — les sélecteurs sont partagés

`[data-role="save"]` et `.consequence-idle` existent aussi dans le panneau
« Classement ». Un locator e2e non scopé prend **le premier du DOM** : le clic
partait sur l'autre bouton, recalculait le classement, et le test lisait *son*
message de conséquence. Aucun POST ne partait vers les tiers, et rien ne le
signalait — le test échouait sur un libellé, symptôme qui n'évoque pas une
erreur de cible.

Tout locator du fichier passe désormais par `#settings-tiers-panel`. La règle
vaut pour les cartes 425 et 426 : **sur cet onglet, un locator non scopé est un
bug en attente.**

### Le fragment ouvert directement au navigateur n'a pas htmx

`page.goto()` sur l'endpoint du panneau rend un HTML sans layout, donc sans
htmx : le script échoue sur `htmx is not defined` et aucun sélecteur ne charge.
Les tests par `requests` ne s'en aperçoivent pas — ils n'exécutent aucun script.
Les tests navigateur passent par `…/admin/settings`, la page par laquelle un
commissaire arrive.

### Le bouton n'est pas un élément htmx

`attendre_cablage` attend un câblage htmx qui n'arrive jamais : l'écouteur est
posé dès l'exécution du script. Ce qu'il faut attendre, c'est le **remplissage
de la carte de collecte** — et comme le panneau charge ses sélecteurs en série,
voir le premier ne dit rien des suivants. Enregistrer trop tôt retomberait sur
les valeurs figées : le test passerait sans avoir rien prouvé.

## Falsification

| Mutation | Effet attendu | Constaté |
|---|---|---|
| Écouteur `inducementPickerChanged` retiré | le clic sur une puce est ignoré | test 1 rouge sur son assertion ; test 2 **vert** — le repli sur les valeurs figées le sauve |
| Charge utile forcée à `inducements: []` | chaque enregistrement vide les coups de pouce | test 2 rouge : « les coups de pouce ont été vidés » |

La première mutation justifie l'existence du second test : **le repli masque la
perte de l'écouteur**, et seul un test qui enregistre sans rien toucher voit le
défaut que la carte annonçait invisible.

Le refus des champs figés a été vérifié à l'écran : un budget forgé à 999 999
renvoie 200 avec « Le champ « budget » du tier « Tier 1 » ne se modifie pas
depuis les réglages », et la base garde 1060.
