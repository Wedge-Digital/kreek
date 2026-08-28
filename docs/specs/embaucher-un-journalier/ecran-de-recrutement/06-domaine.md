# Écran de recrutement · Phase 6 : domaine

**Phase 5** : `05-use-cases.md` · **Conception** : `../00-conception.md`

## Les règles, récapitulées — quatorze

### Le journalier existe

| | Règle |
|---|---|
| E1 | Il naît dans `players` **au début du rapport**, avec `membership: Journeyman` |
| E2 | Il reçoit un **maillot** dès sa naissance |
| E3 | Il est **visible dans l'effectif** et **compte dans la valeur d'équipe** |
| E4 | Il joue, gagne des SPP, prend ses améliorations **comme un joueur ordinaire** |

E4 est le pivot de toute la conception : sans lui, les hausses de valeur du LRB
seraient impossibles à porter — un joueur qui n'existe pas ne peut pas
s'améliorer.

### Le recrutement

| | Règle |
|---|---|
| R1 | Se fait en phase `Recruitment`, avec les autres recrutements |
| R2 | Son prix **est** la valeur courante du joueur |
| R3 | Ne crée rien : il bascule `Journeyman → Active` |
| R4 | La limite de **16 ne compte que les permanents** |
| R5 | Un journalier déjà au panier ne se reprend pas |

### La disparition

| | Règle |
|---|---|
| D1 | Non recruté à la sortie de `Recruitment` → `Dismissed` |
| D2 | Rapport **annulé** → **supprimé**, pas `Dismissed` |
| D3 | Rapport **dépublié** → le `membership` ne bouge pas |

D2 et D1 ne se ressemblent pas, et c'est voulu : un journalier d'un rapport
annulé n'a jamais joué. Le garder en `Dismissed` polluerait l'effectif d'une
trace de rien.

### Les gardes

| | Règle |
|---|---|
| G1 | Un journalier absent de l'effectif chargé est une **ligne rejetée** |
| G2 | Un `JourneymanRecruited` sur un `Dismissed` est **ignoré avec un `WARN`** |

---

## `teams` — trois touches

### 1 · Une méthode d'agrégat, calquée sur sa voisine

```rust
pub fn recruit_journeyman(
    &self,
    player_id: PlayerId,
    cost_kpo: Kpo,
) -> Result<TeamDomainEvent, DomainError> {
    self.expect_phase(GamePhase::Recruitment)?;
    if self.treasury.0 < cost_kpo.0 {
        return Err(DomainError::InsufficientTreasury);
    }
    Ok(TeamDomainEvent::JourneymanRecruited { player_id, cost_kpo })
}
```

**Mêmes gardes que `recruit_player`** : la phase, et la trésorerie.

**Pas de `roster_line` ni de `base_value`.** Le joueur existe déjà et `players`
sait tout de lui ; `teams` ne transporte que ce qu'il décide — l'identité de la
cible et le prix qu'il débite.

C'est le principe déjà écrit sur `PlayerDismissed` : *« `players` possède le
joueur, il sait tout de lui ; ce qu'il ignorait, c'est la décision. »*

### 2 · Le panier — trois variantes, et une règle propre

```rust
pub fn add_journeyman(&mut self, player_id: PlayerId) -> Result<(), DomainError> {
    // R5 — on peut recruter deux Trois-quarts, jamais deux fois le même homme.
    if self.lines.iter().any(|l| matches!(l,
        BasketLine::Journeyman { player_id: p, .. } if p == &player_id)) {
        return Err(DomainError::JourneymanAlreadyInBasket);
    }
    // G1 — la liste des recrutables vient de l'effectif chargé.
    let Some(h) = self.hireable.iter().find(|h| h.player_id == player_id) else {
        return Err(DomainError::JourneymanNoLongerAvailable);
    };
    if self.permanent_count() >= MAX_SQUAD {
        return Err(DomainError::SquadFull);
    }
    …
}
```

**R5 est une règle que les postes n'ont pas.** Un poste est un type — deux
Trois-quarts sont deux joueurs différents. Un journalier est **un homme**, et il
n'y en a qu'un.

**G1 vit ici, dans le domaine, et reste pur** : le panier compare son contenu à
la liste qu'on lui a donnée, il n'interroge rien.

### 3 · La limite de 16 change de définition

```rust
/// R4 — seuls les permanents comptent dans le plafond. Un journalier du panier
/// y entre : il devient permanent. Ceux qui restent n'y sont pas — ils vont
/// partir.
fn permanent_count(&self) -> usize {
    self.squad.permanent_len()
        + self.lines.iter().filter(|l| matches!(l,
            BasketLine::Player { .. } | BasketLine::Journeyman { .. })).count()
}
```

Sans R4, un coach à seize dont trois journaliers ne pourrait recruter personne —
alors que les recruter est précisément ce qui le sortirait de l'impasse.

`Squad` doit donc distinguer ce qu'elle compte : `permanent_len()` exclut les
`is_temporary`, `len()` les inclut. **Deux méthodes, deux questions** — la VE
veut tout le monde, le plafond veut les permanents.

### Deux variantes de `DomainError`, pas une

```rust
pub enum DomainError {
    …,
    JourneymanNoLongerAvailable,   // G1
    JourneymanAlreadyInBasket,     // R5
}
```

Une seule variante — « ce journalier n'est pas recrutable » — serait plus
courte. Mais **les deux causes se corrigent différemment** : l'une demande de
recharger la page, l'autre de regarder son panier. Un message unique enverrait
chercher.

## `players` — la troisième variante et deux événements

```rust
pub enum RosterMembership { Active, Journeyman, Dismissed }
```

Aucune migration : la variante s'ajoute, les 49 262 lignes `Active` ne bougent
pas.

```rust
PlayerBecamePermanent { player_id }   // R3
PlayerJourneymanLost  { player_id }   // D1
```

**`PlayerJourneymanLost` n'est pas `PlayerDismissed`**, et c'est la distinction
la plus importante de cette phase. Un journalier perdu n'a pas été **renvoyé par
une décision du coach** — il n'a simplement pas été retenu.

Les confondre ferait apparaître ces joueurs dans l'historique des renvois, où
ils raconteraient une décision qui n'a jamais été prise.

**D2 n'a pas d'événement** : un journalier d'un rapport annulé est supprimé, pas
marqué. Il n'a pas d'histoire à conserver.

## Ce que le domaine ne fait pas

- **Aucun contrôle d'autorisation** ni d'appartenance à l'équipe : ce sont des
  ports, que le `CLAUDE.md` interdit au domaine.
- **Aucun calcul de prix** : c'est `value_kpo`, lu tel quel (R2).
- **G2 n'est pas dans le domaine** : le garde-fou de `players` s'exécute à la
  réception d'un événement, dans un listener.

## Tests

### L'agrégat

| Test | Règle |
|---|---|
| `recruter_un_journalier_hors_phase_echoue` | R1 |
| `recruter_sans_tresorerie_echoue` | la garde de trésorerie |
| `l_evenement_ne_porte_ni_roster_line_ni_base_value` | R3 — la forme de l'événement |

### Le panier

| Test | Règle |
|---|---|
| `le_meme_journalier_ne_s_ajoute_pas_deux_fois` | **R5** |
| `un_journalier_absent_des_recrutables_est_refuse` | **G1** |
| `deux_journaliers_differents_s_ajoutent` | le cas passant qu'on croirait interdit |
| `seize_permanents_bloquent_le_recrutement` | R4 |
| **`seize_dont_trois_journaliers_autorisent_le_recrutement`** | R4 — le cas qui donne son sens à la règle |
| `un_journalier_du_panier_compte_dans_le_plafond` | R4 — il devient permanent |
| `validate_all_rejette_un_journalier_disparu` | G1 à la validation, pas seulement à l'ajout |

`seize_dont_trois_journaliers_autorisent_le_recrutement` est celui qui compte :
il échoue si quelqu'un « simplifie » `permanent_count` en `squad.len()`, ce qui
compilerait et paraîtrait juste.

### Le statut

| Test | Règle |
|---|---|
| `un_journalier_perdu_n_est_pas_un_renvoye` | D1 — deux événements distincts |
| `l_effectif_inclut_les_journaliers` | E3 |
| `le_plafond_les_exclut` | R4 |

Les deux derniers se contredisent en apparence, et c'est exactement pourquoi ils
doivent coexister : **l'effectif et le plafond ne comptent pas la même chose.**

## Règles métier

**Aucune à préciser.** Les quinze décisions de `00-conception.md` couvrent la
fonctionnalité, et cette phase n'en fait apparaître aucune — elle rend
explicites deux distinctions qui étaient implicites : perdu ≠ renvoyé, et
effectif ≠ plafond.
