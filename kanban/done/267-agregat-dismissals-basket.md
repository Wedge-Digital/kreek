# Agrégat `DismissalsBasket` — le plancher des 11 éligibles

**Priorité : haute**
**Dépend de :** 257, 259
**Bloque :** 268
**Spec :** `docs/specs/phases-recrutement-renvois/renvois/06-domaine.md` §1
**Fichiers :** `src/app/teams/domain/dismissals_basket.rs` (nouveau)

## Problème

Le panier de renvois porte **une seule règle**, mais elle est subtile : on ne peut pas
descendre sous **11 joueurs éligibles au prochain match**.

Toutes les gardes de composition du recrutement — plafond de 16, quota par poste,
limites croisées, trésorerie — sont ici **sans objet** : retirer ne peut violer aucune
borne haute.

## Action

### 1. L'agrégat

```rust
pub struct DismissalsBasket {
    team_id: TeamId,
    version: BasketVersion,
    lines:   Vec<DismissalBasketLine>,   // ← seul état persisté
    squad:   SquadSnapshot,        // hydraté (carte 259)
    catalog: RosterCatalog,        // hydraté — pour le staff possédé
}
```

**Pas de trésorerie** : un renvoi ne rembourse rien, l'agrégat n'a aucune raison de la
connaître.

### 2. Le plancher, précisément

```rust
fn check_eligible_floor(&self, id: &PlayerId) -> Result<(), DomainError> {
    let player = self.squad.find(id).ok_or(DomainError::PlayerNotInSquad)?;
    // Un absent ne compte pas parmi les éligibles : le renvoyer n'entame
    // pas le plancher.
    if !player.available_for_next_match { return Ok(()); }
    if self.eligible_after_basket() <= MIN_ELIGIBLE {
        return Err(DomainError::EligibleFloorReached);
    }
    Ok(())
}
```

`MIN_ELIGIBLE = 11`.

`eligible_after_basket()` compte les membres actifs disponibles **moins les joueurs déjà
marqués** : c'est ce qui fait que le plancher se resserre à chaque marquage, et qu'un
joueur marqué compte encore tant que le lot n'est pas appliqué.

**Un joueur absent reste toujours renvoyable**, quel que soit le nombre d'éligibles.

### 3. Seconde garde : la possession du staff

`check_staff_owned` — ne pas marquer plus que ce que l'équipe possède, lignes déjà en
attente comprises.

### 4. Trois états par ligne

```rust
pub enum DismissalActionState {
    Removable,
    Marked,                          // ← sans équivalent au recrutement
    Blocked { cause: BlockCause },   // EligibleFloor
}
```

`Marked` existe parce qu'une ligne s'annule ici **depuis la ligne du joueur**, pas
seulement depuis le panier.

## Tests unitaires — les 11 de la spec

Les **3, 4 et 6** sont les plus importants : ils couvrent l'interaction entre le
plancher et le contenu du panier, seule vraie subtilité de la page.

- 3 : 12 éligibles, un marqué → le second refuse
- 4 : 9 éligibles → marquer un **absent** passe
- 6 : démarquer rend un éligible et rouvre le marquage

## Checklist

- [ ] Agrégat sans trésorerie, sans `async`, sans port
- [ ] `MIN_ELIGIBLE = 11`, comptage après panier
- [ ] Un absent reste renvoyable quel que soit le compte
- [ ] `check_staff_owned` tient compte des lignes en attente
- [ ] `DismissalActionState` à trois cas
- [ ] `validate_all` refuse en bloc
- [ ] Les 11 tests de `renvois/06-domaine.md` §5
- [ ] `make check-arch` au vert, `make test` au vert
