//! Vérification de cohérence référentielle d'un jeu de données de référence.
//!
//! Un ruleset est fourni par l'exploitant (cf. `REFERENCES__DIR`) : rien ne
//! garantit que ses fichiers se référencent correctement entre eux. Ces
//! contrôles répondent à « les uids cités existent-ils ? », question que le
//! parsing JSON ne pose pas.
//!
//! Fonction pure sur le port : aucune dépendance framework, aucun accès disque.

use crate::app::references::domain::port::IReferenceRepository;

#[derive(Debug, PartialEq, Eq)]
pub enum ConsistencyViolation {
    UnknownSkill { owner: String, uid: String },
    UnknownCategory { owner: String, uid: String },
    UnknownStaff { owner: String, uid: String },
    UnknownSpecialRule { owner: String, uid: String },
    UnknownRoster { owner: String, uid: String },
}

impl std::fmt::Display for ConsistencyViolation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let (kind, owner, uid) = match self {
            ConsistencyViolation::UnknownSkill { owner, uid } => ("compétence", owner, uid),
            ConsistencyViolation::UnknownCategory { owner, uid } => ("catégorie", owner, uid),
            ConsistencyViolation::UnknownStaff { owner, uid } => ("staff", owner, uid),
            ConsistencyViolation::UnknownSpecialRule { owner, uid } => ("règle spéciale", owner, uid),
            ConsistencyViolation::UnknownRoster { owner, uid } => ("roster", owner, uid),
        };
        write!(f, "{owner} référence un(e) {kind} inconnu(e) : {uid}")
    }
}

/// Retourne toutes les incohérences du jeu de données. Vide = jeu cohérent.
pub fn check_consistency(repo: &dyn IReferenceRepository) -> Vec<ConsistencyViolation> {
    let mut out = check_teams(repo);
    out.extend(check_star_players(repo));
    out
}

fn check_teams(repo: &dyn IReferenceRepository) -> Vec<ConsistencyViolation> {
    let mut out = Vec::new();
    for team in repo.list_teams() {
        out.extend(missing_staff(repo, &team.uid, &team.allowed_staff));
        out.extend(missing_special_rules(repo, &team.uid, &team.special_rules));
        for pos in &team.available_players {
            out.extend(missing_skills(repo, &pos.uid, &pos.skills));
            out.extend(missing_categories(repo, &pos.uid, &pos.primary_access));
            out.extend(missing_categories(repo, &pos.uid, &pos.secondary_access));
        }
    }
    out
}

fn check_star_players(repo: &dyn IReferenceRepository) -> Vec<ConsistencyViolation> {
    let mut out = Vec::new();
    for star in repo.list_star_players() {
        out.extend(missing_skills(repo, &star.uid, &star.skills));
        for roster in &star.available_for_rosters {
            if repo.find_team_by_uid(roster).is_none() {
                out.push(ConsistencyViolation::UnknownRoster {
                    owner: star.uid.clone(),
                    uid:   roster.clone(),
                });
            }
        }
    }
    out
}

fn missing_skills(
    repo: &dyn IReferenceRepository,
    owner: &str,
    uids: &[String],
) -> Vec<ConsistencyViolation> {
    uids.iter()
        .filter(|uid| repo.find_skill_by_uid(uid).is_none())
        .map(|uid| ConsistencyViolation::UnknownSkill { owner: owner.to_string(), uid: uid.clone() })
        .collect()
}

fn missing_categories(
    repo: &dyn IReferenceRepository,
    owner: &str,
    uids: &[String],
) -> Vec<ConsistencyViolation> {
    let known = repo.list_skill_categories();
    uids.iter()
        .filter(|uid| !known.iter().any(|c| &c.id == *uid))
        .map(|uid| ConsistencyViolation::UnknownCategory { owner: owner.to_string(), uid: uid.clone() })
        .collect()
}

fn missing_staff(
    repo: &dyn IReferenceRepository,
    owner: &str,
    uids: &[String],
) -> Vec<ConsistencyViolation> {
    let known = repo.list_staff();
    uids.iter()
        .filter(|uid| !known.iter().any(|s| &s.uid == *uid))
        .map(|uid| ConsistencyViolation::UnknownStaff { owner: owner.to_string(), uid: uid.clone() })
        .collect()
}

fn missing_special_rules(
    repo: &dyn IReferenceRepository,
    owner: &str,
    uids: &[String],
) -> Vec<ConsistencyViolation> {
    let known = repo.list_special_rules();
    uids.iter()
        .filter(|uid| !known.iter().any(|r| &r.uid == *uid))
        .map(|uid| ConsistencyViolation::UnknownSpecialRule {
            owner: owner.to_string(),
            uid:   uid.clone(),
        })
        .collect()
}
