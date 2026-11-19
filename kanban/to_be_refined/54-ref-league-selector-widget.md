# BC `références` — Référentiel des ligues + widget sélecteur

**Priorité : haute**
**Dépend de :** rien de nouveau
**Contexte :** `références` — widget HTMX

## Objectif

Exposer le référentiel des ligues (données statiques) et servir un fragment HTMX sélecteur de ligue. Le widget reçoit un `on_select` à l'instanciation : il l'appelle lorsque le coach choisit une ligue. Le BC `références` ne connaît aucune route de `team_creation`.

---

## Conception

### Modèle de données

Les ligues sont des données statiques — pas de table SQL. Elles peuvent être définies en code (tableau de constantes) ou dans un fichier de config chargé au démarrage.

```rust
// références/domain/model/league.rs
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct LeagueId(pub String);   // ex. "WOODLAND_LEAGUE"

pub struct League {
    pub id:   LeagueId,
    pub name: String,
}
```

### Route et paramètres

```
GET /références/leagues/selector
    ?selected={league_id | ""}
    &on_select={url_encodé}          ← callback fourni par la page hôte
```

Le `on_select` est une URL opaque pour le BC `références`. Il ne l'interprète pas — il l'injecte dans chaque option du widget.

### Fragment rendu — 3 états

**État 1 — sélection existante (widget fermé) :**
Affiche le nom de la ligue sélectionnée + son identifiant. Clic → ouvre le dropdown (toggle local, sans requête serveur).

**État 2 — dropdown ouvert :**
Liste toutes les ligues disponibles. La ligue active est cochée. Clic sur une option → POST vers `on_select`.

```html
<!-- Chaque option dans le dropdown -->
<button
  hx-post="{{ on_select }}"
  hx-vals='{"league_id": "{{ league.id }}"}'
  hx-target="closest .league-selector-container"
  hx-swap="outerHTML">
  {{ league.name }}
</button>
```

La réponse du `on_select` (fournie par `team_creation`) doit retourner le widget dans son état fermé mis à jour. `team_creation` peut soit re-appeler le fragment `références` côté serveur, soit retourner un `HX-Trigger` qui déclenche un rechargement du widget.

**État 3 — aucune sélection :**
Trigger affiché en tirets avec texte placeholder.

### Intégration côté page hôte (`team_creation`)

```html
<div id="league-selector-container"
     hx-get="/références/leagues/selector
             ?selected={{ draft.league_id }}
             &on_select=/team-creation/{{ draft_id }}/league"
     hx-trigger="load"
     hx-target="this"
     hx-swap="outerHTML">
</div>
```

---

## Points à préciser

- Liste des ligues disponibles : définies en dur dans le code ou dans un fichier TOML/JSON de config ? Si config, quel mécanisme de rechargement ?
- La réponse du callback `on_select` (côté `team_creation`) re-sert-elle directement le fragment `références` (appel serveur interne), ou retourne-t-elle un `HX-Trigger` pour que le widget se recharge via un second `hx-get` ?
- Faut-il une option "Sans affiliation" dans le référentiel ?

---

## Checklist

- [ ] `LeagueId` newtype + struct `League`
- [ ] Référentiel statique des ligues (constantes ou config)
- [ ] Handler `GET /références/leagues/selector` avec params `selected` + `on_select`
- [ ] Template `league-selector-fragment.html` — 3 états (fermé / ouvert / vide)
- [ ] Toggle ouvert/fermé sans requête serveur (JS inline ou hyperscript)
- [ ] Route déclarée dans le router `références`
