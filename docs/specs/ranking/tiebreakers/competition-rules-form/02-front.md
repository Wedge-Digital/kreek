# Phase 2 — Architecture front (`competition-rules-form`)

## Constat de départ

La page phase 2 de création de compétition (`new-competition-phase-2.html`) n'est
**pas** une page d'assemblage à widgets : c'est un formulaire monolithique dont tout
l'état vit en JS, sérialisé par `buildJSON()` et envoyé en **un seul POST JSON** sur
la route courante (`routes.rs:8`). La section « Ordre des critères de départage » est
rendue côté client depuis la const `TIEBREAK_CRITERIA` (ligne 164) et réhydratée
depuis `INITIAL_RULES` par `initFromExistingRules()`.

## Décisions

### D1 — Pas de nouveau widget

La section des départages **reste une partie du formulaire phase 2**.

Raison : son état doit être lu au moment du submit global par la page hôte, or les
widgets ne s'appellent jamais entre eux (CLAUDE.md, « Communication par événements
DOM »). Un widget autonome imposerait un auto-save séparé sur un brouillon de
compétition, donc deux chemins de persistance concurrents pour un même formulaire.

Conséquence : aucun endpoint GET/POST nouveau, aucun `hx-disinherit`, aucun événement
DOM. La refonte de la page entière en assemblage à widgets reste possible plus tard,
hors périmètre de cette feature.

**Widgets existants vérifiés** (`src/app/*/io/web/widgets/`) : aucun réutilisable —
rien ne traite d'une liste ordonnable ni d'un catalogue de critères.

### D2 — Catalogue servi par `ranking` via port + adapter

Le catalogue des 8 critères (id + libellé + ordre canonique) appartient au BC
`ranking`. `competitions` le consulte en lecture synchrone :

```
competitions/ports.rs          ← trait ITiebreakCatalogPort + DTO
infrastructure/competitions/   ← adapter, seul à importer ranking
main.rs                        ← instancie l'adapter, injecte dans le contexte competitions
```

Le handler de la page injecte le catalogue dans le template en JSON. La const
`TIEBREAK_CRITERIA` codée en dur **disparaît** du template, libellés compris.

À noter : cela crée une dépendance `ranking` ↔ `competitions` dans les deux sens
(`ranking` consulte déjà les règles de `competitions` via `competition_info_adapter`).
Les deux passent par des ports, sans import direct — pas de violation `check-arch`.

## Séparation front / back

| Interaction | Où | Détail |
|---|---|---|
| Cocher / décocher un critère | **Front** | Bascule le flag `activated` **sur place** — la position dans la liste n'est pas touchée, ce qui satisfait la règle 2 par construction |
| Glisser un critère | **Front** | Réordonne, y compris un critère décoché |
| Renumérotation | **Front** | Seuls les critères actifs sont numérotés 1..N ; les inactifs affichent un tiret |
| Garde-fou « au moins un coché » | **Front + domaine** | Front : message inline + bouton d'enregistrement désactivé (confort). Domaine : validation autoritaire (correction) |
| Persistance | **Back** | Aucun aller-retour HTTP avant le submit global — le POST existant est simplement étendu |

## État client

`criteriaOrder` passe d'une liste de `{ id, label }` à une liste de
`{ id, label, activated }`, alimentée par le catalogue injecté et par la configuration
existante. L'ordre de la liste **est** l'ordre de priorité, activation comprise.

Rendu d'une ligne (calqué sur la maquette validée `app-league-rules.html`) :

```html
<label class="tiebreak-row [is-off]" draggable="true" data-id="...">
  <input type="checkbox" class="tiebreak-check" [checked]>
  <div class="tiebreak-rank">N | —</div>
  <span class="tiebreak-drag">⠿</span>
  <span class="tiebreak-label">…</span>
</label>
```

La ligne devient un `<label>` (et non un `<div>` comme dans la maquette) : le libellé
entier devient cliquable et l'avertissement « missing associated label » disparaît.
Le `.tiebreak-drag` reste la poignée visuelle ; `draggable` porte sur la ligne.

## Hydratation d'une configuration existante

`initFromExistingRules()` doit reconstruire ordre **et** activation. Le catalogue
étant la source de vérité de la liste des critères, un critère présent au catalogue
mais absent de la configuration sauvegardée est ajouté en fin de liste, **actif**
(règle 3).

Aucune reprise de données à prévoir : le projet n'est pas en production, il n'existe
pas de configuration antérieure à préserver.

## Événements DOM

Aucun. La section ne publie ni ne consomme d'événement.

## Actions HTTP

| Action | Route | Changement |
|---|---|---|
| Enregistrer les règles | POST route phase 2 existante | Payload étendu : `additionnal_ranking_points` doit porter ordre **et** activation. Forme exacte arbitrée en phase 4 |

## Règles métier — état

Les 5 règles validées en phase 1 sont dans `../README.md`. Précisions apportées par
cette phase :

- **Règle 2** satisfaite par construction : décocher ne déplace pas la ligne.
- **Règle 4** (gel après démarrage) : **aucun travail requis**. Le formulaire de règles
  n'existe que dans le parcours de création — pas de route d'édition après création,
  pas de `settings_tab.rs` dans `io/web/admin/`, maquette `app-competition-admin-settings.html`
  non implémentée. La règle devient une contrainte à respecter le jour où la page
  d'admin des règles sera construite. La carte séparée envisagée en phase 1 est retirée.

Aucun point ouvert à l'issue de cette phase.
