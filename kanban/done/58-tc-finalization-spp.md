# BC `team_creation` — Dépense de SPP en phase de finalisation

**Priorité : haute**
**Dépend de :** `55-ref-skill-picker-widget.md`, `56-tc-player-identity.md`
**Contexte :** `team_creation` — action coach

## Objectif

Permettre au coach de dépenser un **pool de SPP équipe** (acquisition de compétences sur les joueurs de son choix) pendant la phase de finalisation, et d'annuler ses choix avant soumission. Les compétences acquises sont incluses dans `TeamSubmitted`.

## Fonctionnement du pool

Les SPP ne sont **pas attribués par joueur** : ils forment un pool global de l'équipe, défini par les règles de création (`CreationRules`). Le coach peut dépenser librement ce pool sur n'importe quel joueur. Le panneau gauche affiche un badge "N compét." uniquement pour les joueurs qui ont déjà reçu une compétence depuis le pool ; le compteur global (panneau droit, carte D) reflète les SPP restants dans le pool équipe.

---

## Conception

### Value objects

```rust
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SkillId(pub String);   // ex. "block", "sure_hands"

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct SppAmount(pub u8);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AcquisitionMode { Chosen, Random }

/// Pool SPP de l'équipe, initialisé depuis CreationRules à la création du draft.
/// Pas de compteur par joueur — c'est un bien commun de l'équipe.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct SppPool(pub u8);

impl SppPool {
    pub fn spend(&self, cost: SppAmount) -> Result<Self, DomainError> {
        self.0.checked_sub(cost.0)
            .map(SppPool)
            .ok_or(DomainError::InsufficientSpp)
    }
    pub fn refund(&self, cost: SppAmount) -> Self {
        SppPool(self.0.saturating_add(cost.0))
    }
}
```

### Événements domaine

```rust
pub struct CreationSppSpent {
    pub team_id:   TeamId,
    pub player_id: PlayerId,
    pub skill_id:  SkillId,
    pub mode:      AcquisitionMode,
    pub spp_cost:  SppAmount,
}

pub struct CreationSppCancelled {
    pub team_id:   TeamId,
    pub player_id: PlayerId,
    pub skill_id:  SkillId,
    pub spp_refund: SppAmount,
}
```

### Commandes et use cases

```rust
pub struct SpendCreationSppCommand {
    pub team_id:   TeamId,
    pub player_id: PlayerId,
    pub skill_id:  SkillId,
    pub mode:      AcquisitionMode,
}

pub struct CancelCreationSppCommand {
    pub team_id:   TeamId,
    pub player_id: PlayerId,
    pub skill_id:  SkillId,
}
```

Le use case `spend_creation_spp` :
1. Charger le draft
2. Vérifier que le joueur a assez de SPP (coût depuis le référentiel `références`)
3. Vérifier que la compétence n'est pas déjà acquise
4. Appender `CreationSppSpent` + mettre à jour la projection
5. Persister
6. Retourner les fragments de mise à jour

### Routes

```
POST   /team-creation/{draft_id}/players/{player_id}/skills
Body : { "skill_id": "sure_hands", "mode": "chosen" }

DELETE /team-creation/{draft_id}/players/{player_id}/skills/{skill_id}
```

### Réponse — fragments OOB

Les deux endpoints retournent les mêmes 4 fragments via OOB swaps :

| Fragment | Cible | Contenu |
|---|---|---|
| Badge SPP joueur | `#player-row-{id}` | SPP restants du joueur (panneau gauche) |
| Header joueur | `#skill-header` | Compteur SPP + liste compétences existantes (carte D) |
| Badge total | `#spp-total-badge` | Total SPP restants dans le header du panneau gauche |
| Synthèse SPP | `#spp-summary` | Table récapitulative mise à jour (carte F) |

Le widget skill-picker (carte E, BC `références`) est rechargé via un événement JS `skillsUpdated` dispatché par le handler, avec les nouveaux paramètres (`spp` et `acquired` mis à jour) :

```rust
// Dans la réponse HTTP
.header("HX-Trigger", serde_json::json!({
    "skillsUpdated": {
        "roster_line_id": player.roster_line_id,
        "spp": player.spp_remaining,
        "acquired": player.acquired_skill_ids(),
        "on_acquire": on_acquire_url,
        "on_cancel":  on_cancel_url
    }
}).to_string())
```

Le container `#skill-picker-container` écoute cet événement pour se recharger depuis `références`.

### Template — synthèse SPP (carte F)

Fragment `spp-summary-fragment.html` :

```html
{% if spp_log.is_empty() %}
  <div class="spp-empty-state">Aucune compétence acquise pour l'instant.</div>
{% else %}
  <table class="spp-summary-table">
    {% for entry in spp_log %}
    <tr>
      <td>
        <div style="display:flex;align-items:center;gap:10px;">
          <div class="jersey-badge">{{ entry.jersey }}</div>
          <div>
            <div class="summary-player">{{ entry.player_name_or_placeholder }}</div>
            <div class="summary-pos">{{ entry.position_name }}</div>
          </div>
        </div>
      </td>
      <td><strong>{{ entry.skill_name }}</strong></td>
      <td><span class="skill-type-pill ...">{{ entry.skill_category }}</span></td>
      <td><span class="mode-chip ...">{{ entry.mode_label }}</span></td>
      <td><span class="summary-cost">{{ entry.spp_cost }} SPP</span></td>
      <td>
        <button hx-delete="{{ routes.cancel_spp(draft_id, entry.player_id, entry.skill_id) }}"
                hx-confirm="Annuler cette compétence ?">
          ✕ Annuler
        </button>
      </td>
    </tr>
    {% endfor %}
  </table>
{% endif %}
```

---

## Points à préciser

- Où est définie la valeur initiale du pool dans `CreationRules` ? Champ à ajouter (`spp_pool: u8`) ou dérivé d'une autre règle ?
- Le use case `spend_creation_spp` doit-il valider que la compétence est accessible pour ce `roster_line_id` (cohérence avec le widget `références`), ou fait-on confiance au widget ?
- `HX-Trigger` pour recharger le skill-picker : le container doit porter `hx-trigger="skillsUpdated from:body"` avec `hx-vals` dynamiques — vérifier la compatibilité avec la version HTMX utilisée.

---

## Checklist

- [ ] `SkillId`, `SppAmount`, `AcquisitionMode` value objects
- [ ] `DomainError::InsufficientSpp`, `DomainError::SkillAlreadyAcquired`
- [ ] `CreationSppSpent` + `CreationSppCancelled` events
- [ ] `SpendCreationSppCommand` + use case (validation SPP + doublon + persist)
- [ ] `CancelCreationSppCommand` + use case (remboursement SPP + persist)
- [ ] Route `SPEND_SPP` (POST) + `CANCEL_SPP` (DELETE) dans `routes.rs` + `router.rs`
- [ ] Handler POST : 4 fragments OOB + `HX-Trigger skillsUpdated`
- [ ] Handler DELETE : mêmes 4 fragments OOB + `HX-Trigger skillsUpdated`
- [ ] Template `spp-summary-fragment.html` (carte F)
- [ ] Template `skill-header-fragment.html` mis à jour avec compteur SPP (carte D)
- [ ] `#skill-picker-container` écoute `skillsUpdated` dans le template de la page
