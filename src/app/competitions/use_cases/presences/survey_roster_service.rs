//! Le roster de la campagne — qui l'on sollicite, et qui n'a pas d'adresse.
//!
//! # Pourquoi un service, et pas un handler
//!
//! Deux ports rendent des DTO — `TeamInfoDto` et `SpaceMemberDto` — et le
//! `CLAUDE.md` interdit qu'ils atteignent un handler ou un gabarit. Ce fichier
//! est le seul à les connaître ; tout ce qui sort d'ici est un objet du domaine
//! local.
//!
//! # Ce qu'il ne fait pas
//!
//! **Il ne classe pas les réponses.** `colonnes` demande à l'agrégat quelles
//! réponses sont présentes, absentes ou silencieuses, et se contente de les
//! joindre à l'affichage. Refiltrer sur `Presence` ici ferait dire R5 à deux
//! endroits, et le jour où la règle bouge, l'un des deux resterait en arrière.
//!
//! **Aucun `.ok()?`.** Le `CLAUDE.md` cite nommément ce mécanisme comme celui
//! qui a fait disparaître un roster sans une ligne de journal. Une équipe
//! escamotée ici, c'est une équipe qui manque au tirage sans que personne sache
//! pourquoi — donc une équipe dont le coach n'est plus membre de l'espace entre
//! dans la campagne comme les autres, avec `email: None`.

use crate::app::competitions::domain::presence_survey::{
    Destinataire, Presence, PresenceSurvey, Repondant, Reponse,
};
use crate::app::competitions::ports::{ICompetitionSpaceMemberPort, ITeamInfoPort, TeamInfoDto};
use crate::app::shared_kernel::bloodbowl::team::TeamId;
use crate::app::shared_kernel::identity::ids::{CoachId, SpaceId};
use std::collections::HashMap;

/// Une équipe engagée, vue par la campagne — le croisement des deux ports.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EquipeSollicitee {
    pub team_id: TeamId,
    pub team_name: String,
    pub coach_id: CoachId,
    /// « Lepandawan · 2 équipes », ou « Ghorak » quand il n'en engage qu'une.
    ///
    /// Construit ici et non dans le gabarit : « 2 équipes » suppose de savoir
    /// combien d'équipes ce coach engage **dans cette saison**, ce qui est une
    /// question sur le roster de la campagne, pas sur la ligne affichée.
    pub coach_label: String,
    /// `None` quand aucun membre de l'espace ne correspond au coach — R3.
    ///
    /// **Le défaut de correspondance entre les deux listes *est* le compte
    /// « sans adresse connue ».** Tenir une seconde liste à côté demanderait de
    /// la garder d'accord avec la première, et rien ne le vérifierait.
    pub email: Option<String>,
}

impl EquipeSollicitee {
    fn a_une_adresse(&self) -> bool {
        self.email.is_some()
    }
}

/// Les équipes que la campagne sollicite, dans l'ordre où le port les rend.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct RosterDeCampagne {
    equipes: Vec<EquipeSollicitee>,
}

impl RosterDeCampagne {
    pub fn equipes(&self) -> &[EquipeSollicitee] {
        &self.equipes
    }

    /// Ce que `PresenceSurvey::ouvrir` attend — R1, une réponse par équipe.
    ///
    /// **Toutes les équipes, adresse ou non.** R3 : refuser le lancement ferait
    /// dépendre une campagne de quatorze coachs de la fiche incomplète d'un seul.
    pub fn destinataires(&self) -> Vec<Destinataire> {
        self.equipes
            .iter()
            .map(|e| Destinataire {
                team_id: e.team_id,
                coach_id: e.coach_id,
            })
            .collect()
    }

    /// R3 — le nombre d'équipes dont le coach n'a pas d'adresse connue. L'écran
    /// l'annonce avant le lancement ; il ne l'empêche pas.
    pub fn sans_adresse(&self) -> usize {
        self.equipes.iter().filter(|e| !e.a_une_adresse()).count()
    }

    /// Les coachs **distincts** : un coach à deux équipes ne compte qu'une fois,
    /// et c'est ce que « 14 équipes, 11 coachs » veut dire à l'écran.
    pub fn coachs(&self) -> usize {
        self.equipes
            .iter()
            .map(|e| e.coach_id)
            .collect::<std::collections::HashSet<_>>()
            .len()
    }

    pub fn equipe(&self, team_id: &TeamId) -> Option<&EquipeSollicitee> {
        self.equipes.iter().find(|e| &e.team_id == team_id)
    }
}

/// Croise les équipes engagées et les membres de l'espace.
///
/// `Result<_, String>` parce que c'est ce que `find_enrolled_teams` rend, comme
/// `load_enrolled_teams` du même BC. `list_space_members`, lui, ne rend pas de
/// `Result` : un espace sans membres est une liste vide, pas une erreur, et R3
/// s'en accommode — toutes les équipes seront simplement sans adresse.
// arch:no-instrument — service d'hydratation : croise deux ports, sans intention métier
pub async fn charger(
    season_id: &str,
    space_id: &SpaceId,
    teams: &dyn ITeamInfoPort,
    members: &dyn ICompetitionSpaceMemberPort,
) -> Result<RosterDeCampagne, String> {
    let engagees = teams.find_enrolled_teams(season_id).await?;
    let adresses = adresses_par_coach(members, space_id).await;
    let equipes_par_coach = compter_par_coach(&engagees);

    let equipes = engagees
        .iter()
        .filter_map(|t| solliciter(t, &adresses, &equipes_par_coach))
        .collect();
    Ok(RosterDeCampagne { equipes })
}

async fn adresses_par_coach(
    members: &dyn ICompetitionSpaceMemberPort,
    space_id: &SpaceId,
) -> HashMap<String, String> {
    members
        .list_space_members(space_id)
        .await
        .into_iter()
        .filter(|m| !m.email.trim().is_empty())
        .map(|m| (m.coach_id, m.email))
        .collect()
}

fn compter_par_coach(engagees: &[TeamInfoDto]) -> HashMap<&str, usize> {
    let mut comptes: HashMap<&str, usize> = HashMap::new();
    for t in engagees {
        *comptes.entry(t.coach_id.as_str()).or_insert(0) += 1;
    }
    comptes
}

/// **Le seul endroit où une équipe peut être écartée**, et seulement pour un
/// identifiant illisible — ce que le dépôt ne produit pas. Pas de `.ok()?`
/// silencieux : le `filter_map` ne masque qu'un cas impossible, jamais une
/// donnée manquante.
fn solliciter(
    t: &TeamInfoDto,
    adresses: &HashMap<String, String>,
    equipes_par_coach: &HashMap<&str, usize>,
) -> Option<EquipeSollicitee> {
    let combien = equipes_par_coach
        .get(t.coach_id.as_str())
        .copied()
        .unwrap_or(1);
    Some(EquipeSollicitee {
        team_id: TeamId::try_new(&t.team_id).ok()?,
        team_name: t.team_name.clone(),
        coach_id: CoachId::try_new(&t.coach_id).ok()?,
        coach_label: label_du_coach(&t.coach_name, combien),
        email: adresses.get(&t.coach_id).cloned(),
    })
}

/// « Lepandawan · 2 équipes » quand il en engage plusieurs, son nom nu sinon.
fn label_du_coach(coach_name: &str, equipes: usize) -> String {
    if equipes > 1 {
        format!("{coach_name} · {equipes} équipes")
    } else {
        coach_name.to_string()
    }
}

// ── La jointure avec les réponses ────────────────────────────────────────────

/// Une ligne de l'onglet : ce que la campagne sait de l'équipe, joint à ce que
/// les deux ports en disent.
///
/// Elle ne porte pas d'initiales : deux lettres tirées d'un nom d'équipe est une
/// décision de mise en forme — si le résultat était faux, on corrigerait la vue
/// — et elle se prend avec l'écran, pas ici. `team_name` suffit à la produire.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LignePresence {
    pub team_id: String,
    pub team_name: String,
    pub coach_label: String,
    /// Vide tant que l'équipe n'a pas répondu.
    pub repondu_le: Option<String>,
    /// R6 — le badge « saisi par vous » : après le tirage, quand une rencontre
    /// est contestée, « qui a dit qu'il venait » a deux réponses possibles.
    pub saisi_par_admin: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct TroisColonnes {
    pub presents: Vec<LignePresence>,
    pub absents: Vec<LignePresence>,
    pub sans_reponse: Vec<LignePresence>,
}

/// **L'agrégat classe, le service joint.** Les trois listes viennent de
/// `presents()`, `absents()` et `sans_reponse()` ; ce qu'on ajoute ici est le nom
/// de l'équipe et le libellé du coach, que la campagne ne connaît pas.
pub fn colonnes(roster: &RosterDeCampagne, survey: &PresenceSurvey) -> TroisColonnes {
    TroisColonnes {
        presents: lignes(roster, &survey.presents()),
        absents: lignes(roster, &survey.absents()),
        sans_reponse: lignes(roster, &survey.sans_reponse()),
    }
}

fn lignes(roster: &RosterDeCampagne, reponses: &[&Reponse]) -> Vec<LignePresence> {
    reponses.iter().map(|r| ligne(roster, r)).collect()
}

/// Une réponse dont l'équipe a quitté la saison depuis l'ouverture garde sa
/// ligne : elle a répondu, et l'effacer de l'écran ferait disparaître une
/// réponse que le tirage écartera de toute façon par R18, avec son motif.
fn ligne(roster: &RosterDeCampagne, reponse: &Reponse) -> LignePresence {
    let connue = roster.equipe(reponse.team_id());
    LignePresence {
        team_id: reponse.team_id().to_string(),
        team_name: connue
            .map(|e| e.team_name.clone())
            .unwrap_or_else(|| "Équipe désengagée".to_string()),
        coach_label: connue.map(|e| e.coach_label.clone()).unwrap_or_default(),
        repondu_le: horodatage(reponse),
        saisi_par_admin: saisie_par_l_organisateur(reponse),
    }
}

/// La date de réponse, ou rien tant que l'équipe est silencieuse. `Presence`
/// rend impossible une déclaration sans horodatage, donc il n'y a pas de
/// troisième cas à traiter.
fn horodatage(reponse: &Reponse) -> Option<String> {
    match reponse.presence() {
        Presence::SansReponse => None,
        Presence::Declaree { le, .. } => Some(le.to_string()),
    }
}

/// R6 — l'organisateur seul porte le badge. Le jeton et l'encart sont tous deux
/// « le coach a répondu », et la table ne les distingue pas non plus (R28).
fn saisie_par_l_organisateur(reponse: &Reponse) -> bool {
    matches!(
        reponse.presence(),
        Presence::Declaree {
            par: Repondant::Organisateur(_),
            ..
        }
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::competitions::domain::match_day::{
        MatchDay, MatchDayName, MatchDayPosition, MatchDayType,
    };
    use crate::app::competitions::domain::presence_survey::{
        AutoRemind, EtatJournee, SurveyDeadline, SurveyId, Venue,
    };
    use crate::app::competitions::ports::{SpaceMemberDto, TeamEnrollmentDto};
    use crate::app::shared_kernel::bloodbowl::date_string::DateString;
    use crate::app::shared_kernel::bloodbowl::ids::{MatchId, SeasonId};
    use crate::app::shared_kernel::identity::authorization::SpaceProfile;
    use crate::app::shared_kernel::identity::space_definition::SpaceDefinition;
    use async_trait::async_trait;

    // ── Les deux ports, en double ────────────────────────────────────────────

    struct FauxTeams(Vec<TeamInfoDto>);

    #[async_trait]
    impl ITeamInfoPort for FauxTeams {
        async fn find_enrolled_teams(&self, _season: &str) -> Result<Vec<TeamInfoDto>, String> {
            Ok(self
                .0
                .iter()
                .map(|t| TeamInfoDto {
                    team_id: t.team_id.clone(),
                    team_name: t.team_name.clone(),
                    coach_id: t.coach_id.clone(),
                    coach_name: t.coach_name.clone(),
                    roster_name: t.roster_name.clone(),
                    logo_url: t.logo_url.clone(),
                })
                .collect())
        }
        async fn find_team_names(&self, _ids: &[String]) -> Result<Vec<TeamInfoDto>, String> {
            Ok(vec![])
        }
        async fn find_team_enrollment(
            &self,
            _team_id: &str,
        ) -> Result<Option<TeamEnrollmentDto>, String> {
            Ok(None)
        }
    }

    struct FauxMembres(Vec<SpaceMemberDto>);

    #[async_trait]
    impl ICompetitionSpaceMemberPort for FauxMembres {
        async fn list_space_members(&self, _space: &SpaceId) -> Vec<SpaceMemberDto> {
            self.0
                .iter()
                .map(|m| SpaceMemberDto {
                    coach_id: m.coach_id.clone(),
                    coach_name: m.coach_name.clone(),
                    email: m.email.clone(),
                })
                .collect()
        }
        async fn find_member_profile(
            &self,
            _coach: &CoachId,
            _space: &SpaceId,
        ) -> Option<SpaceProfile> {
            None
        }
        async fn find_all_spaces(&self) -> Vec<SpaceDefinition> {
            vec![]
        }
    }

    // ── Fabriques ────────────────────────────────────────────────────────────

    fn equipe(nom: &str, coach: &CoachId, coach_name: &str) -> TeamInfoDto {
        TeamInfoDto {
            team_id: TeamId::new().to_string(),
            team_name: nom.to_string(),
            coach_id: coach.to_string(),
            coach_name: coach_name.to_string(),
            roster_name: "Humains".to_string(),
            logo_url: None,
        }
    }

    fn membre(coach: &CoachId, nom: &str, email: &str) -> SpaceMemberDto {
        SpaceMemberDto {
            coach_id: coach.to_string(),
            coach_name: nom.to_string(),
            email: email.to_string(),
        }
    }

    async fn charger_avec(
        equipes: Vec<TeamInfoDto>,
        membres: Vec<SpaceMemberDto>,
    ) -> RosterDeCampagne {
        charger(
            "01KZVCKDG19DXZHJA295WSJGMV",
            &SpaceId::new(),
            &FauxTeams(equipes),
            &FauxMembres(membres),
        )
        .await
        .expect("chargement")
    }

    fn campagne(roster: &RosterDeCampagne) -> PresenceSurvey {
        let round = MatchDay {
            id: MatchId::new(),
            season_id: SeasonId::new(),
            name: MatchDayName::try_new("Journée 3".to_string()).unwrap(),
            day_type: MatchDayType::FixedDate,
            date_start: None,
            date_end: None,
            position: MatchDayPosition::try_new(2).unwrap(),
            pairings: vec![],
        };
        PresenceSurvey::ouvrir(
            SurveyId::new(),
            SeasonId::new(),
            &round,
            &roster.destinataires(),
            SurveyDeadline::try_new("2026-10-10".to_string()).unwrap(),
            AutoRemind::new(true),
            &DateString::try_new("2026-10-01".to_string()).unwrap(),
        )
        .expect("ouverture")
    }

    fn date(s: &str) -> DateString {
        DateString::try_new(s.to_string()).unwrap()
    }

    fn vierge() -> EtatJournee {
        EtatJournee {
            figee: false,
            rencontres: vec![],
        }
    }

    // ── Le croisement ────────────────────────────────────────────────────────

    #[tokio::test]
    async fn un_coach_a_deux_equipes_porte_le_compte_sur_les_deux_lignes() {
        let coach = CoachId::new();
        let roster = charger_avec(
            vec![
                equipe("Les Crocs du Chaos", &coach, "Lepandawan"),
                equipe("Étoiles de Naggaroth", &coach, "Lepandawan"),
            ],
            vec![membre(&coach, "Lepandawan", "lepandawan@example.test")],
        )
        .await;

        assert_eq!(roster.equipes().len(), 2);
        assert_eq!(roster.coachs(), 1, "un coach distinct, deux équipes");
        for e in roster.equipes() {
            assert_eq!(e.coach_label, "Lepandawan · 2 équipes");
        }
    }

    #[tokio::test]
    async fn un_coach_a_une_seule_equipe_n_affiche_que_son_nom() {
        let coach = CoachId::new();
        let roster = charger_avec(
            vec![equipe("Bordeciel FC", &coach, "Ghorak")],
            vec![membre(&coach, "Ghorak", "ghorak@example.test")],
        )
        .await;

        assert_eq!(roster.equipes()[0].coach_label, "Ghorak");
    }

    // ── R3 — un coach sans adresse n'empêche pas le lancement ────────────────

    #[tokio::test]
    async fn un_coach_sans_adresse_entre_quand_meme_dans_la_campagne() {
        let (avec, sans) = (CoachId::new(), CoachId::new());
        let roster = charger_avec(
            vec![
                equipe("Rats des Égouts", &avec, "Skreek"),
                equipe("Nains Rouges", &sans, "Borin"),
            ],
            vec![membre(&avec, "Skreek", "skreek@example.test")],
        )
        .await;

        assert_eq!(roster.sans_adresse(), 1);
        assert_eq!(
            roster.destinataires().len(),
            2,
            "R3 — son équipe est sollicitée comme les autres, seul l'e-mail manque"
        );
    }

    /// Une adresse vide en base vaut une absence d'adresse : `SpaceMemberDto`
    /// porte un `String` et non un `Option`, donc c'est ici que le cas se règle.
    #[tokio::test]
    async fn une_adresse_vide_compte_comme_absente() {
        let coach = CoachId::new();
        let roster = charger_avec(
            vec![equipe("Gobelins Éclatés", &coach, "Nifk")],
            vec![membre(&coach, "Nifk", "   ")],
        )
        .await;

        assert_eq!(roster.sans_adresse(), 1);
        assert_eq!(roster.equipes()[0].email, None);
    }

    /// Elle n'est pas escamotée : un `.ok()?` ici ferait manquer une équipe au
    /// tirage sans une ligne de journal.
    #[tokio::test]
    async fn une_equipe_dont_le_coach_a_quitte_l_espace_reste_sollicitee() {
        let parti = CoachId::new();
        let roster = charger_avec(
            vec![equipe("Morts-vivants de Khemri", &parti, "Setep")],
            vec![],
        )
        .await;

        assert_eq!(roster.equipes().len(), 1);
        assert_eq!(roster.sans_adresse(), 1);
        assert_eq!(roster.destinataires().len(), 1);
    }

    // ── La jointure : l'agrégat classe, le service joint ─────────────────────

    #[tokio::test]
    async fn les_trois_colonnes_suivent_la_classification_de_l_agregat() {
        let (a, b, c) = (CoachId::new(), CoachId::new(), CoachId::new());
        let roster = charger_avec(
            vec![
                equipe("Les Présents", &a, "Alpha"),
                equipe("Les Absents", &b, "Beta"),
                equipe("Les Muets", &c, "Gamma"),
            ],
            vec![
                membre(&a, "Alpha", "a@example.test"),
                membre(&b, "Beta", "b@example.test"),
                membre(&c, "Gamma", "c@example.test"),
            ],
        )
        .await;
        let mut survey = campagne(&roster);
        let admin = CoachId::new();
        survey
            .enregistrer(
                &roster.equipes()[0].team_id,
                Venue::Presente,
                Repondant::Organisateur(admin),
                &vierge(),
                &date("2026-10-05"),
            )
            .unwrap();
        survey
            .enregistrer(
                &roster.equipes()[1].team_id,
                Venue::Absente,
                Repondant::Jeton,
                &vierge(),
                &date("2026-10-06"),
            )
            .unwrap();

        let cols = colonnes(&roster, &survey);

        assert_eq!(cols.presents.len(), 1);
        assert_eq!(cols.absents.len(), 1);
        assert_eq!(cols.sans_reponse.len(), 1);
        assert_eq!(cols.presents[0].team_name, "Les Présents");
        assert_eq!(cols.presents[0].coach_label, "Alpha");
        assert_eq!(cols.presents[0].repondu_le.as_deref(), Some("2026-10-05"));
        assert!(
            cols.presents[0].saisi_par_admin,
            "R6 — le badge « saisi par vous » distingue une réponse posée d'une reçue"
        );
        assert!(!cols.absents[0].saisi_par_admin, "celle-là vient du jeton");
        assert_eq!(cols.sans_reponse[0].repondu_le, None);
    }

    /// Une équipe désengagée depuis l'ouverture garde sa ligne : elle a répondu,
    /// et l'effacer ferait disparaître une réponse que R18 écartera du tirage
    /// avec son motif.
    #[tokio::test]
    async fn une_reponse_dont_l_equipe_a_quitte_la_saison_garde_sa_ligne() {
        let coach = CoachId::new();
        let roster = charger_avec(
            vec![equipe("Partants", &coach, "Delta")],
            vec![membre(&coach, "Delta", "d@example.test")],
        )
        .await;
        let survey = campagne(&roster);

        let cols = colonnes(&RosterDeCampagne::default(), &survey);

        assert_eq!(cols.sans_reponse.len(), 1);
        assert_eq!(cols.sans_reponse[0].team_name, "Équipe désengagée");
    }
}
