# Le tirage au sort du MVP

**Épic :** E21 — Le mode de saisie et l'après-match en séquence · **Ordre :** 8 · **Dépend de :** 637
**Conception :** `docs/specs/saisie-mobile/E21-cadrage.md` (R8, § 3, § 8.4, § 8.5, § 8.6)
**Maquettes :** `assets/rawpages/html/app-mobile-solo-mvp-home.html`, `app-mobile-solo-mvp-home-drawn.html`

## Objectif

L'étape MVP offre un second moyen : **désigner six joueurs** qui ont joué, et
le serveur en **tire un au sort**. « Tirer » reste fermé tant qu'il n'y a pas
six désignés. Le résultat est définitif ; l'écran dit « Tiré au sort parmi
Thorgrim, Borin, Durin, Balin, Nori et Gimli ».

## Conception

**Domaine**

```rust
pub struct DesignatedPlayer { pub player: ActionPlayer, pub display_name: String } // arch:ok instantané

MatchReportDomainEvent::MvpDrawn { action_id: ActionId, team_side: TeamSide,
    designated: Vec<DesignatedPlayer> }

pub fn draw_mvp(&self, side: TeamSide, designated: Vec<DesignatedPlayer>, rng: &mut impl Rng,
    action_id: ActionId, recorded_by: CoachId)
    -> Result<(Self, Vec<MatchReportDomainEvent>), DomainError>;
```

Exactement six désignés, distincts ; mêmes refus que `record_mvp` (637), qu'il
réutilise pour l'action tirée. Rend `ActionRecorded` (le `Mvp` au tour 16) puis
`MvpDrawn`, appendés ensemble par `append_many`. L'agrégat retient les MVP tirés
et leurs désignés.

**L'aléa** — décision du 2026-11-01 : le précédent du tirage du calendrier,
`rng: &mut impl Rng` reçu, `StdRng::from_os_rng()` dans le contrôleur, graine
fixe en test (`competitions/domain/draw.rs:319`).

**« Définitif », même en correction** — décision du 2026-11-01 :
`delete_action` refuse un MVP tiré (`DrawnMvpIsFinal`), et le journal ne lui
met pas de ✕.

**Route** `POST /{id}/mvp/{side}/draw`, dans `routes_de_saisie()` : six couples
`player_id`/`player_type`. Les noms sont résolus comme pour une action. L'onglet
« 🎲 Tirage au sort » s'ajoute à la page de 637, coché par défaut comme sur la
maquette.

**Rien ne sort du BC** : à la publication, le MVP tiré est une action `Mvp`
comme une autre. `MvpDrawn` est nommé dans l'aiguillage du publisher (631).

**Au journal** — décision du 2026-11-01 : `match_report_actions.draw_size
SMALLINT` nullable, posé par `MvpDrawn`, pour afficher « tiré parmi 6
désignés ».

## Données de production

Un événement **ajouté** ; la colonne nullable `draw_size`, vide pour
l'existant. Ce qui sort du BC reste une action `Mvp` ; « tiré parmi » pour les
seuls nouveaux.

## Checklist

- [ ] `DesignatedPlayer`, `MvpDrawn`, `draw_mvp`, son rejeu ; `delete_action` refuse un MVP tiré
- [ ] Use case et route du tirage ; l'onglet sur la page MVP
- [ ] Migration : `draw_size` ; projection de `MvpDrawn` ; « tiré parmi » au journal
- [ ] Tests unitaires, graine fixe : moins de six ou un doublon refusés ; le tiré est l'un des six ; l'action tirée est au tour 16 ; rejeu de `MvpDrawn`
- [ ] E2E : désigner six joueurs, tirer, l'action « tiré parmi » au journal ; « Tirer » fermé à cinq
- [ ] `tests/impact-map.toml` mis à jour
- [ ] `make lint`, `make check-arch`, `make test`, e2e impactés
