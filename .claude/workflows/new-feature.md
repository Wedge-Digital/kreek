# Workflow « Nouvelle fonctionnalité »

Ce workflow gouverne la création de nouvelles fonctionnalités. Il est **activé à la demande** par l'utilisateur ("on suit le workflow feature" ou "active le workflow"). Il n'est PAS utilisé pour les corrections de bugs, les refactos isolées, ou les modifications mineures.

Chaque phase produit un livrable validé par l'utilisateur avant de passer à la suivante. On ne code pas avant la phase 6.

---

## Organisation et progression

### Granularité : page par page

Les phases 2 à 8 s'appliquent **page par page** (ou onglet par onglet), pas à la fonctionnalité entière. Seule la Phase 1 (maquettes) peut couvrir plusieurs pages d'un coup pour avoir une vision d'ensemble.

### Fichiers de spec

Chaque fonctionnalité a un dossier de spec :

```
docs/specs/<feature>/
├── README.md                ← index, statut par page
├── 01-<page-name>.md        ← phases 2-8 pour cette page
├── 02-<page-name>.md
└── ...
```

Le `README.md` contient le tableau de progression :

```markdown
| Page | Maquette | Front | Back | DTOs | Use cases | Domaine | Intégration | Cartes |
|---|---|---|---|---|---|---|---|---|
| Dashboard | ✅ | ✅ | | | | | | |
| Inscriptions | ✅ | | | | | | | |
```

### Contexte minimal

Quand on attaque une page, on lit **uniquement** son fichier de spec et le README — pas les specs des autres pages. Cela permet de travailler sans surcharger le contexte.

---

## Phase 1 — Design (maquettes)

**Entrée** : besoin utilisateur exprimé en texte
**Contexte à charger** : design system (`assets/static/css/`), direction artistique, maquettes existantes (`assets/rawpages/`), principes UX du projet
**Granularité** : toutes les pages de la fonctionnalité (vue d'ensemble)

### Processus

1. Comprendre le besoin : poser des questions si nécessaire
2. Identifier les écrans / pages concernés par la fonctionnalité
3. Créer une maquette HTML/CSS statique pour chaque écran dans `assets/rawpages/html/`
4. Présenter les maquettes à l'utilisateur
5. Itérer par boucles successives sur le feedback : ajustements visuels, interactions, états (vide, chargé, erreur, succès)
6. Valider l'ensemble des maquettes

### Sortie

Ensemble de maquettes HTML/CSS validées dans `assets/rawpages/html/`.

### Règles

- Les maquettes utilisent le design system existant (shared.css, variables CSS)
- Chaque état est maquetté : état vide, état chargé, état d'erreur, état de succès
- Un fichier par page/onglet
- On ne passe aux phases suivantes qu'après validation de toutes les maquettes

---

## Phase 2 — Architecture front (spec de composition)

**Entrée** : maquette validée de la page
**Granularité** : une page à la fois

### Processus

Pour la page en cours :

1. **Identifier les widgets** qui composent la page (sections autonomes avec leur propre endpoint)
2. **Définir la communication** entre widgets :
   - Quels événements DOM chaque widget émet (`htmx.trigger(document.body, 'eventName', payload)`)
   - Quels événements chaque widget écoute (`hx-trigger="eventName from:body"`)
   - Le payload de chaque événement (quels champs, quels types)
3. **Séparer front vs back** :
   - Ce qui est géré côté front (JS/Alpine inline dans le widget) : toggle, filtres locaux, drag & drop
   - Ce qui est envoyé au back (HTMX `hx-get`/`hx-post`) : chargement de données, mutations
4. **Modes d'interaction** de chaque widget : lecture seule, édition (auto-save), sélection, mutation
5. **Widgets existantes réutilisables** : vérifier dans `src/app/*/io/web/widgets/` et `src/app/*/io/web/templates/widgets/`

### Sortie

Section "Phase 2 — Architecture front" dans le fichier de spec de la page :

```
| Widget | BC | Endpoint | Trigger | Émet | Mode |
|--------|-----|----------|---------|------|------|
| ...    | ... | ...      | ...     | ...  | ...  |

Événements :
- eventName : { payload } — émis par [widget], écouté par [widgets]

Actions :
- POST /endpoint — description → HX-Trigger: eventName
```

### Règles

- Chaque widget respecte les conventions CLAUDE.md (hx-disinherit, Alpine lifecycle, CSS embarqué)
- Aucune communication directe entre widgets — uniquement via événements DOM sur `body`
- La page hôte est un assemblage pur : quasi zéro JS, pas de logique métier

---

## Phase 3 — Architecture back (organisation des traitements)

**Entrée** : spec front validée de la page
**Granularité** : une page à la fois

### Processus

1. **Mapper widgets → BCs** : quel BC fournit chaque widget
2. **Vérifier les widgets existantes** : grep dans les fichiers widgets existants
3. **Définir les fichiers** :
   - Un fichier handler par widget dans `widgets/` (convention `_widget.rs`)
   - Un fichier template par widget dans `templates/widgets/`
   - Les routes dans `routes.rs`
4. **Identifier les ports nécessaires** : si un widget a besoin de données d'un autre BC
5. **Identifier les domain services** : si un handler a besoin de transformer des DTOs de port en objets domaine

### Sortie

Section "Phase 3 — Architecture back" dans le fichier de spec avec le plan de fichiers.

### Règles

- Un BC ne fournit jamais la widget d'un autre BC
- Les données inter-BC passent par des ports (cf. CLAUDE.md « Adapters inter-BCs »)
- Les routes cross-BC sont accédées via `AppRoutes` (cf. CLAUDE.md « Accès aux routes »)

---

## Phase 4 — Contrats de données (DTOs entrée/sortie)

**Entrée** : plan back validé de la page
**Granularité** : une page à la fois

### Processus

Pour chaque handler identifié :

1. **DTO d'entrée** : struct `Deserialize` pour le body/query params
   - Côté command (POST/PUT/DELETE) : utiliser des value objects via smart constructors
   - Côté query (GET) : types primitifs acceptables
2. **DTO de sortie** : template struct Askama avec VMs
   - VMs purs domaine : constructeur `from_domain()` co-localisé
   - VMs dépendant du port : fonctions dans `builders.rs`
   - VMs suffixés `Vm` (convention)
3. **DTOs de port** : si un nouveau port est nécessaire, définir les structs dans `ports.rs`

### Sortie

Section "Phase 4 — DTOs" dans le fichier de spec avec les structs Rust.

### Règles

- Les commandes utilisent des value objects, jamais des primitives nues (cf. CLAUDE.md)
- Les VMs de lecture peuvent utiliser des primitives
- Les DTOs de port ne sont jamais exposés aux handlers (cf. CLAUDE.md « Domain services »)

---

## Phase 5 — Use cases (couche commande)

**Entrée** : DTOs et commandes définis pour la page
**Granularité** : une page à la fois

### Processus

Pour chaque mutation (POST/PUT/DELETE) :

1. Créer le fichier use case (`_use_case.rs`)
2. Définir la signature : commande en entrée, résultat en sortie
3. Implémenter l'orchestration :
   - Charger l'agrégat depuis le repository
   - Charger les données externes via les ports (si nécessaire)
   - Appeler la méthode métier sur l'agrégat (définie en phase 6)
   - Persister les modifications
   - Émettre les événements
4. Définir les erreurs applicatives (enum)

### Sortie

Section "Phase 5 — Use cases" dans le fichier de spec avec les signatures et l'orchestration.

### Règles

- Le use case ne contient pas de logique métier — il coordonne (cf. CLAUDE.md « Responsabilités des couches »)
- Le use case ne connaît pas HTTP, HTML, ni les formats de sérialisation
- Un use case par mutation (pas de use case "fourre-tout")

---

## Phase 6 — Domaine (logique métier)

**Entrée** : use cases définis pour la page
**Granularité** : une page à la fois

### Processus

1. Implémenter les méthodes domaine sur les agrégats (appelées par les use cases)
2. Créer les value objects nécessaires avec smart constructors
3. Définir les erreurs domaine (`DomainError`)
4. Écrire les tests unitaires pour chaque règle métier

### Sortie

Section "Phase 6 — Domaine" dans le fichier de spec. Code domaine testé (`cargo test` passe).

### Règles

- Le domaine n'importe jamais de crate framework (axum, sqlx, etc.)
- Toute logique "est-ce autorisé ?" vit dans le domaine, pas dans le use case ni le handler
- Tests unitaires obligatoires pour chaque règle métier

---

## Phase 7 — Effets de bord (persistance, événements, réponses)

**Entrée** : domaine implémenté et testé pour la page
**Granularité** : une page à la fois

### Processus

1. **Persistance** : implémenter les méthodes repository (si nouvelles)
2. **Événements** : câbler les événements domaine → app event bus → listeners inter-BC
3. **Handlers** : implémenter les handlers HTTP (le code est minimal — construction de commande + appel use case + rendu template)
4. **Templates** : intégrer les maquettes de la phase 1 dans les templates Askama
5. **Tests E2E** : écrire les tests Playwright pour le parcours complet
6. **Validation manuelle** : tester dans le navigateur

### Sortie

Code complet, testé (unitaire + E2E), commité.

### Règles

- Les handlers sont des traducteurs HTTP — pas de logique métier (cf. CLAUDE.md)
- Avant de supprimer du code : vérifier qu'il n'est utilisé nulle part (règle 4)
- Quand on déplace du code : copier-coller exact (règle 5)
- Commit après validation utilisateur (règle 1)

---

## Phase 8 — Cartes kanban

**Entrée** : spec complète de la page (phases 2-7)
**Granularité** : une page à la fois

### Processus

À partir de la spec complète de la page, produire un ensemble de **cartes kanban** dans `kanban/ready_to_be_done/` :

1. Découper l'implémentation en cartes indépendantes et ordonnées
2. Chaque carte correspond à une unité livrable (compilable, testable, commitable)
3. Chaque carte contient : objectif, dépendances, conception détaillée, checklist
4. Les cartes suivent le cycle de vie kanban (cf. CLAUDE.md)

### Sortie

Cartes kanban créées dans `kanban/ready_to_be_done/`. Le fichier de spec de la page est marqué comme complet dans le README.

### Règles de découpage

- Une carte par widget (handler + template + route)
- Une carte par use case / mutation complexe
- Une carte pour les value objects et méthodes domaine
- Une carte pour les tests E2E de la page
- Les cartes sont ordonnées par dépendance (domaine → use case → handler → template)
- Chaque carte est réalisable en une session de travail
