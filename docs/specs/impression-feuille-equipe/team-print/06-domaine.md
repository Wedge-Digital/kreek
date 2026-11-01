# Page « Imprimer » — Phase 6 : domaine

Entrée : `05-use-cases.md`. **Aucun nouvel agrégat, aucune mutation.** Quatre
ajouts : une méthode sur `Team`, trois modules de fonctions pures.

## Récapitulatif des règles métier (validé)

| # | Règle | Où elle vit |
|---|---|---|
| R1 | Seule une équipe prête à jouer est imprimable ; sinon pas de bouton, et la page explique | teams · `Team::is_printable` |
| R2 | Prochain adversaire : prochain match de la saison d'inscription (unique) ; sans match à venir, option grisée avec son motif | competitions |
| R3 | Autre équipe : compétition → saison → équipe, même espace, équipes prêtes à jouer | routes existantes |
| R4 | VE imprimée : la VE courante, celle que le rapport figera — journaliers compris | teams (valeur persistée) |
| R5 | Colonne SPP : le solde disponible | players (`spp_remaining`) |
| R6 | Les journaliers sont imprimés, marqués comme tels | players, match_report |
| R7 | Coups de pouce : palier de **sa** compétition, prix de **son** roster ; « — » si non disponible | references (règles existantes) |
| R8 | Trésorerie : sur toutes les fiches, sans option (révisée au test manuel) | présentation |
| R9 | Rappels du MVP et des fans, liste des météos : lus dans le corpus ; absents s'il ne les fournit pas | references |
| R10 | Favori : la VE la plus forte ; à égalité, l'équipe à domicile. Domicile inconnu (autre équipe choisie à la main) et VE égales : personne, la ligne se remplit à la main *(révisée en revue de la PR #12)* | match_report · fonction pure |
| R11 | Petite monnaie imprimée : l'écart de VE, avec le rappel des dépenses du favori | match_report · fonction pure |
| R12 | Prochain match : celui en cours de saisie s'il existe, sinon le premier à venir dans l'ordre des journées. Le coach peut choisir un autre match à venir (carte 610) | competitions · fonction pure |
| R13 | Star player disponible pour un roster : sans liste de rosters autorisés, ou si le roster y figure | references · déplacé du contrôleur |
| R14 | Joueur alignable : ce que le domaine déclare tel (`presence.alignable()`), jamais un recomptage | teams, players |
| R15 | Lignes vides : 16 − lignes imprimées ; masquer les indisponibles en ajoute | présentation |
| R16 | Tout utilisateur connecté peut imprimer, comme il peut consulter la fiche | `require_auth` (existant) |

Hors domaine : densité compacte automatique (navigateur), découpage des
annexes en pages et dédoublonnage des compétences (présentation, service).

## Forme du domaine (validée)

### match_report — `domain/underdog.rs`

Extrait de `MatchReportPreMatch::topdog_team_id` et `petty_cash` **par
copier-coller** (règle 5), puis adapté pour prendre des valeurs au lieu des
champs de l'agrégat.

```rust
pub enum Side { Home, Away }

/// Favori : la VE la plus forte ; à égalité, l'équipe à domicile (R10).
pub fn top_dog(home_tv: TeamValue, away_tv: TeamValue) -> Side;

/// Base de la petite monnaie : l'écart de VE (R11).
pub fn tv_gap(home_tv: TeamValue, away_tv: TeamValue) -> u32;
```

`MatchReportPreMatch::topdog_team_id` et `petty_cash` appellent ces deux
fonctions. Leur comportement ne change pas, `expect` compris : y toucher est
hors périmètre.

### teams — `Team::is_printable` (R1)

```rust
impl Team {
    /// Vrai pour une équipe inscrite et prête à jouer — la condition même du
    /// bouton « Imprimer » de la fiche (`BannerVm::pour_etat`).
    pub fn is_printable(&self) -> bool;
}
```

### competitions — `domain/next_opponent.rs` (R2, R12)

```rust
pub enum PairingStatus { InProgress, Upcoming, Played }

pub struct ScheduledPairing {
    pub round_id: MatchId,
    pub round_position: MatchDayPosition,
    pub pairing: Pairing,
    pub status: PairingStatus,
}

pub struct NextOpponent {
    pub team_id: TeamId,
    pub round_id: MatchId,
    pub opponent_is_home: bool,
}

pub fn next_opponent(team: &TeamId, pairings: &[ScheduledPairing]) -> Option<NextOpponent>;

/// Tous les matchs à venir dans l'ordre de R12 ; le premier est `next_opponent`.
pub fn upcoming_opponents(team: &TeamId, pairings: &[ScheduledPairing]) -> Vec<NextOpponent>;
```

`PairingStatus::from_projection` ne connaît que les trois statuts que la
projection contient (`in_progress`, `upcoming`, `completed`) ; un statut inconnu
rend `None`, et la recherche répond « calendrier illisible » au lieu de sauter la
ligne *(revue de la PR #12)*.

Réutilise `Pairing::adversaire_de` (`match_day.rs:134`). Le service construit
les `ScheduledPairing` depuis `PairingDisplayDto` ; la fonction ne dépend pas de
l'ordre de la requête SQL.

### references — `domain/star_player_availability.rs` (R13)

Déplacé de `inducement_selector_controller.rs:183-188` par copier-coller.

```rust
pub fn star_player_available_for(star: &StarPlayer, roster_id: &str) -> bool;
```

Le contrôleur `inducement_selector_controller` l'appelle désormais : une seule
définition de la règle.

## Erreurs domaine

Aucune nouvelle : les fonctions pures sont totales (elles rendent un `Option`
ou une valeur), `is_printable` un booléen.

## Tests unitaires prévus

| Règle | Cas |
|---|---|
| R10 | VE plus forte à domicile · à l'extérieur · égalité |
| R11 | écart dans les deux sens · écart nul |
| R12 | un match en cours passe avant un match à venir · matchs joués ignorés · position la plus basse d'abord · aucun match · équipe absente des appariements |
| R13 | liste vide · roster présent · roster absent |
| R1 | prête à jouer · en recrutement · non inscrite |
| non-régression | les tests existants de `MatchReportPreMatch` et du sélecteur de coups de pouce restent verts |
