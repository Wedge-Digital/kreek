# Classement — Phase 6 : Domaine

## Récapitulatif exhaustif des règles métier (phases 1 à 5)

1. Le classement est trié par points de classement décroissants uniquement — pas de tri interactif utilisateur
2. La colonne "Bonus" est masquée dans cette feature, quelles que soient les règles de la compétition (calcul des points bonus hors scope)
3. Le widget affiche 2 états vides distincts : "Aucune équipe dans la compétition." (aucune équipe inscrite) vs "Aucun match n'a encore été joué." (équipes inscrites, aucune ligne de classement)
4. Le widget affiche un état d'erreur explicite si les règles de classement de la saison ne sont pas configurées — jamais un classement à 0 partout ni un état vide
5. Une équipe n'a qu'une seule ligne "active" par saison à tout instant : la plus récente par **ordre d'enregistrement global** (pas par journée). Une équipe peut avoir plusieurs lignes pour la même journée (plusieurs matchs le même jour de calendrier, ou — à terme — une correction de rapport qui ajoute une nouvelle ligne sans modifier l'ancienne)
6. Pas de déduplication ni de notion d'idempotence à gérer à l'écriture — chaque event traité insère une nouvelle ligne
7. `ranking` ne parle jamais directement à `teams` — uniquement à `competitions`
8. Un `MatchReportPublished` génère toujours exactement 2 lignes de classement (une par équipe), jamais 0, jamais 1, et elles sont écrites **atomiquement** (jamais l'une sans l'autre)
9. Les noms d'équipe ne sont jamais stockés sur une ligne de classement — toujours résolus à la lecture via le port
10. Si les règles ne sont pas configurées au moment de la publication du rapport, **aucune ligne n'est créée** pour ce match (pas de valeur par défaut, pas de calcul partiel)
11. La ligne de classement stocke les compteurs cumulés (matches joués, victoires, nuls, défaites, points de classement) — pas seulement les points
12. Le rang (position dans le classement) est calculé à la lecture, jamais stocké sur la ligne

**Vois-tu des règles métier manquantes ou à corriger ?**

---

## Méthodes domaine

Le domaine `ranking` n'a pas d'agrégat mutable au sens classique (pas d'entité chargée puis modifiée en mémoire) — une ligne de classement est un **fait immuable** : une fois enregistrée, elle n'est jamais modifiée. La logique métier est donc une fonction de construction pure : *étant donné la ligne précédente d'une équipe (ou son absence), le résultat du match, et les règles de la compétition, produire la nouvelle ligne*.

```rust
// domain/ranking_line.rs
pub struct RankingLine {
    pub team_id:          TeamId,
    pub season_id:        SeasonId,
    pub round_id:          RoundId,
    pub match_report_id:   MatchReportId,
    pub recorded_at:        DateTime<Utc>,
    pub matches_played:     u32,
    pub wins:                u32,
    pub draws:               u32,
    pub losses:              u32,
    pub ranking_points:      RankingPoints,
}

pub enum MatchOutcome { Win, Draw, Loss }

impl RankingLine {
    /// Dérive le résultat d'une équipe à partir des deux scores — total,
    /// jamais d'erreur (toute paire de scores produit un résultat valide).
    pub fn derive_outcome(own_score: MatchScore, opponent_score: MatchScore) -> MatchOutcome {
        match own_score.into_inner().cmp(&opponent_score.into_inner()) {
            Ordering::Greater => MatchOutcome::Win,
            Ordering::Equal   => MatchOutcome::Draw,
            Ordering::Less    => MatchOutcome::Loss,
        }
    }

    /// Construit la nouvelle ligne de classement d'une équipe après un match.
    /// `previous` : dernière ligne connue de cette équipe pour cette saison (`None` = première apparition).
    pub fn record_match(
        previous: Option<&RankingLine>,
        team_id: TeamId,
        season_id: SeasonId,
        round_id: RoundId,
        match_report_id: MatchReportId,
        recorded_at: DateTime<Utc>,
        outcome: MatchOutcome,
        rules: &RankingRules,
    ) -> RankingLine {
        let base = previous.map(|p| (p.matches_played, p.wins, p.draws, p.losses, p.ranking_points))
            .unwrap_or((0, 0, 0, 0, RankingPoints(0)));
        let (matches_played, wins, draws, losses, points) = base;
        let match_points = match outcome {
            MatchOutcome::Win  => rules.win_points,
            MatchOutcome::Draw => rules.draw_points,
            MatchOutcome::Loss => rules.lose_points,
        };
        RankingLine {
            team_id, season_id, round_id, match_report_id, recorded_at,
            matches_played: matches_played + 1,
            wins:   wins   + if matches!(outcome, MatchOutcome::Win)  { 1 } else { 0 },
            draws:  draws  + if matches!(outcome, MatchOutcome::Draw) { 1 } else { 0 },
            losses: losses + if matches!(outcome, MatchOutcome::Loss) { 1 } else { 0 },
            ranking_points: points + match_points,
        }
    }
}
```

Le use case (Phase 5) appelle `derive_outcome` + `record_match` deux fois : une fois pour l'équipe domicile (`own = home_score`, `opponent = away_score`), une fois pour l'équipe visiteuse (inversé) — jamais de logique de calcul dupliquée entre les deux appels.

## Value objects

| VO | Définition | Invariant |
|---|---|---|
| `MatchScore` | newtype `u8` | Aucun — tout score est valide (pas de borne métier connue) |
| `RankingPoints` | newtype `u32`, `Add` dérivé/implémenté | Aucun — cumul non borné |

Pas de `nutype` avec `validate(...)` nécessaire ici : ces deux types n'ont aucun invariant à protéger, seulement à sortir du régime "primitif nu" (règle CQRS). `RankingLine`, `MatchOutcome`, `RankingRules` sont des structs/enum domaine ordinaires (pas des VOs à proprement parler).

## Erreurs domaine

Aucune. Le calcul est **total** : toute combinaison (ligne précédente ou absence, scores, règles) produit une ligne de classement valide. Aucune règle métier identifiée ne peut faire échouer `record_match` ou `derive_outcome`. (`RecordMatchRankingError::RulesNotConfigured`, Phase 5, est une erreur applicative — absence de règles côté port — pas une erreur domaine.)

## Tests unitaires prévus

| Règle | Test |
|---|---|
| #1 (dérivation résultat) | `derive_outcome` : score supérieur → Win, égal → Draw, inférieur → Loss (3 tests) |
| #8 (2 lignes par match) | `record_match` appelé pour home et away avec les scores inversés produit bien 2 résultats symétriques (si home Win alors away Loss, jamais les deux Win) |
| #11 (cumul) | `record_match` avec `previous = Some(ligne existante)` additionne correctement matches_played/wins/draws/losses/points à l'existant, ne les remplace pas |
| #11 (première ligne) | `record_match` avec `previous = None` part de zéro (pas de panique, pas de valeur négative) |
| Cumul multi-lignes | 3 appels successifs de `record_match` (Win puis Draw puis Loss) sur la même équipe produisent les cumuls attendus à chaque étape |
| Points appliqués | `record_match` utilise bien `rules.win_points`/`draw_points`/`lose_points` selon l'issue — pas de valeur en dur |
