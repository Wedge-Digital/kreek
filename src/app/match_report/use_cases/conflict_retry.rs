//! Le réessai unique d'une commande d'un camp sur conflit de version (carte 653).
//!
//! En « Chacun son équipe », les deux coachs écrivent au même instant dans le
//! même flux, chacun dans son camp : l'event store refuse le second append
//! (`RepositoryError::ConcurrentWrite`), sans dire qui a gagné la course ni ce
//! qu'il a écrit. Rejouer la commande sur l'agrégat relu est **sûr pour les
//! seules commandes que le domaine protège** d'une seconde écriture du même
//! camp — une action s'ajoute, un MVP ou des coups de pouce déjà saisis sont
//! refusés. C'est la liste blanche de la décision du 2026-11-11 (cadrage E22,
//! § 12.2) : `record_action`, `delete_action`, `record_mvp`, `draw_mvp`,
//! `record_inducements`, `init_temp_players`, et depuis la carte 654
//! `record_team_fan_factor` — le fan factor d'un seul camp, dont le domaine
//! refuse un second jet.
//!
//! Le fan factor des deux camps (« Je saisis tout ») et l'après-match n'y
//! entrent pas tant qu'ils écrivent les deux camps d'un coup : leur domaine
//! écrase sans refuser, et un réessai y serait un « dernier qui écrit gagne »
//! silencieux (carte 658 pour l'après-match).
//!
//! Un seul réessai : deux conflits de suite disent que l'état change sous la
//! main, et l'utilisateur doit le voir — 409 et rechargement, comme en E21.
//!
//! ## Pourquoi deux tours écrits à la main
//!
//! La première forme prenait l'essai en fermeture `AsyncFnMut`. Le compilateur
//! refuse alors le futur du contrôleur axum (« implementation of `Send` is not
//! general enough ») : le futur d'une fermeture async qui emprunte des
//! `&dyn Port` n'est pas prouvé `Send` pour toute durée de vie. Chaque use case
//! écrit donc ses deux tours, et ce module ne garde que la décision et le
//! journal :
//!
//! ```ignore
//! let first = record_once(&cmd, repo, bus).await;
//! if !lost_the_race(COMMAND, &first) {
//!     return first;
//! }
//! settle(COMMAND, record_once(&cmd, repo, bus).await)
//! ```
//!
//! Chaque essai refait tout depuis le chargement de l'agrégat : rien de ce
//! qu'un essai perdu a calculé — budget, trésorerie, valeurs d'équipe — ne
//! survit au suivant.

/// Le premier essai, puis le seul réessai.
pub(crate) const MAX_ATTEMPTS: u32 = 2;

/// Une erreur de use case qui sait dire si elle vient d'un conflit de version.
pub(crate) trait ConflictAware {
    fn is_conflict(&self) -> bool;
}

/// Vrai si le premier essai a perdu la course : il faut le rejouer. Le dit
/// au journal.
///
/// Une erreur qui n'est pas un conflit — un refus du domaine, un port en
/// panne — ne se rejoue pas.
pub(crate) fn lost_the_race<T, E: ConflictAware>(
    command: &'static str,
    first: &Result<T, E>,
) -> bool {
    let lost = matches!(first, Err(e) if e.is_conflict());
    if lost {
        tracing::info!(
            command,
            max_attempts = MAX_ATTEMPTS,
            "conflit de version, relecture et nouvelle tentative"
        );
    }
    lost
}

/// L'issue du réessai, rendue telle quelle. Un second conflit se dit au
/// journal : c'est lui que le client recevra en 409.
pub(crate) fn settle<T, E: ConflictAware>(
    command: &'static str,
    second: Result<T, E>,
) -> Result<T, E> {
    if matches!(&second, Err(e) if e.is_conflict()) {
        tracing::info!(
            command,
            max_attempts = MAX_ATTEMPTS,
            "second conflit de version, rendu au client"
        );
    }
    second
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug, PartialEq)]
    enum Outcome {
        Conflict,
        Other,
    }

    impl ConflictAware for Outcome {
        fn is_conflict(&self) -> bool {
            matches!(self, Outcome::Conflict)
        }
    }

    /// Les deux tours tels que les use cases les écrivent, sur un script
    /// d'issues : rend l'issue finale et le nombre d'essais.
    fn run(script: Vec<Result<u8, Outcome>>) -> (Result<u8, Outcome>, usize) {
        let mut issues = script.into_iter();
        let mut calls = 0;
        let mut attempt = || {
            calls += 1;
            issues.next().expect("essai de trop")
        };
        let first = attempt();
        let result = if lost_the_race("test", &first) {
            settle("test", attempt())
        } else {
            first
        };
        (result, calls)
    }

    #[test]
    fn a_first_success_is_not_retried() {
        assert_eq!(run(vec![Ok(1)]), (Ok(1), 1));
    }

    #[test]
    fn a_conflict_is_retried_once_and_the_retry_wins() {
        assert_eq!(run(vec![Err(Outcome::Conflict), Ok(2)]), (Ok(2), 2));
    }

    #[test]
    fn two_conflicts_return_the_conflict() {
        let script = vec![Err(Outcome::Conflict), Err(Outcome::Conflict)];
        assert_eq!(run(script), (Err(Outcome::Conflict), 2));
    }

    #[test]
    fn a_non_conflict_error_is_not_retried() {
        assert_eq!(run(vec![Err(Outcome::Other)]), (Err(Outcome::Other), 1));
    }

    /// Le refus du réessai sort tel quel : c'est le domaine qui parle, pas un
    /// conflit.
    #[test]
    fn the_retry_error_comes_out_as_is() {
        let script = vec![Err(Outcome::Conflict), Err(Outcome::Other)];
        assert_eq!(run(script), (Err(Outcome::Other), 2));
    }
}
