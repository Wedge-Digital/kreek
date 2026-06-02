# BC `références` — Widget catalogue de compétences par ligne de roster

**Priorité : haute**
**Dépend de :** référentiel rosters existant
**Contexte :** `références` — widget HTMX

## Objectif

Servir le fragment "catalogue de compétences" utilisé lors de la phase de finalisation d'équipe. Le widget reçoit en paramètre la ligne de roster concernée, le budget SPP disponible, les compétences déjà acquises, et un callback à appeler lors de l'acquisition. Il ne connaît ni `team_creation` ni l'identité du joueur.

---

## Conception

### Route et paramètres

```
GET /références/roster-lines/{roster_line_id}/skill-picker
    ?spp={n}                         ← budget disponible (fourni par team_creation)
    &acquired={skill_id,skill_id,...} ← compétences déjà possédées (grise les doublons)
    &on_acquire={url_encodé}         ← callback POST pour acquérir une compétence
    &on_cancel={url_encodé}          ← callback DELETE pour annuler une acquisition
```

### Modèle de données

Les compétences sont des données statiques, indexées par ligne de roster.

```rust
// références/domain/model/skill.rs
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SkillId(pub String);   // ex. "block", "dodge", "sure_hands"

pub struct Skill {
    pub id:          SkillId,
    pub name:        String,
    pub description: String,
    pub category:    SkillCategory,   // General | Strength | Agility | Passing | Mutation
    pub nature:      SkillNature,     // Normal | Elite
}

pub enum SkillCategory { General, Strength, Agility, Passing, Mutation }
pub enum SkillNature   { Normal, Elite }

// Coûts SPP selon le mode d'acquisition BB2020
pub struct SppCost {
    pub chosen_normal:  u8,   // 3
    pub chosen_elite:   u8,   // 6
    pub random_normal:  u8,   // 2
    pub random_elite:   u8,   // 4
}
```

Les catégories **accessibles** à une ligne de roster (Primary / Secondary) sont définies dans la définition du roster. Le widget filtre la liste et met en évidence les catégories Primary.

### Fragment rendu

**Filtres :**
- Pills "Type" : affiche uniquement les catégories accessibles à ce `roster_line_id` (grises les autres)
- Pills "Nature" : Normal / Élite
- Toggle "Mode" : Choisie / Aléatoire — change les coûts affichés en temps réel (JS inline)

**Table de compétences :**
Chaque ligne : nom + description, type, nature, coût SPP (selon mode actif), bouton d'action.

Règles d'affichage :
- Compétence déjà dans `acquired` → ligne désactivée (`opacity: 0.45`, bouton grisé "Déjà acquise")
- Coût > `spp` → ligne désactivée, bouton "Budget insuf."
- Sinon → bouton actif "Choisir" ou "Tirer au sort" selon le mode

```html
<!-- Bouton d'acquisition -->
<button
  hx-post="{{ on_acquire }}"
  hx-vals='{"skill_id": "{{ skill.id }}", "mode": "chosen"}'
  hx-target="#skill-picker-container"
  hx-swap="outerHTML"
  {% if skill.cost > spp or skill.id in acquired %}disabled{% endif %}>
  + Choisir
</button>
```

Le widget se re-rend entier après acquisition (paramètres mis à jour par `team_creation` dans la réponse du callback).

### Intégration côté page hôte (`team_creation`)

Chargé lors de la sélection d'un joueur via l'événement `playerSelected` :

```html
<div id="skill-picker-container"
     hx-get="/références/roster-lines/skill-picker"
     hx-trigger="playerSelected from:body"
     hx-vals="js:{
       roster_line_id: event.detail.roster_line_id,
       spp:            event.detail.spp,
       acquired:       event.detail.acquired,
       on_acquire:     event.detail.on_acquire,
       on_cancel:      event.detail.on_cancel
     }"
     hx-target="this"
     hx-swap="outerHTML">
  <!-- État vide : "Sélectionnez un joueur" -->
</div>
```

---

## Points à préciser

- Les compétences sont-elles dans un fichier JSON/TOML statique ou dans une table SQL `références` ? (cohérence avec le référentiel rosters existant)
- Le mode Aléatoire implique-t-il un tirage côté serveur ou côté client ? (la plateforme est un registre — le coach tire physiquement, la règle actuelle semble être "saisie libre")
- Les catégories Primary/Secondary par ligne de roster sont-elles déjà modélisées dans le référentiel existant ou à ajouter ?

---

## Checklist

- [ ] `SkillId`, `Skill`, `SkillCategory`, `SkillNature` dans le domaine `références`
- [ ] `SppCost` : coûts par mode et nature
- [ ] Association ligne-de-roster → catégories accessibles (Primary/Secondary)
- [ ] Handler `GET /références/roster-lines/{rl_id}/skill-picker` avec les 4 params
- [ ] Template `skill-picker-fragment.html` : filtres + toggle mode + table
- [ ] JS inline pour le toggle Choisie/Aléatoire (recalcul des coûts + labels boutons)
- [ ] Désactivation des lignes (doublon acquis, budget insuffisant)
- [ ] Route déclarée dans le router `références`
