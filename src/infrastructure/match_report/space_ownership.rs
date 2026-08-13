//! `match_report` répond sur ses rapports (carte 319).
//!
//! Un seul résolveur : `match_report_proj` porte `space_id`, donc comparaison
//! directe, sans saut.
//!
//! Les paramètres `{pairing_id}` et `{action_id}` de ce BC n'en reçoivent pas :
//! ils n'apparaissent jamais seuls, toujours accompagnés du
//! `{match_report_id}` qui, lui, est contrôlé. Contrôler le parent suffit.
//!
//! Ses quatre routes portant `{team_id}` seront couvertes par le résolveur de
//! `teams` (carte 320) — la liste du middleware étant plate, un BC bénéficie
//! des résolveurs des autres sans les connaître.

use crate::app::match_report::domain::match_report_repository_port::IMatchReportRepository;
use crate::app::shared_kernel::identity::ids::SpaceId;
use crate::web::middleware::space_scope::ISpaceOwnership;
use async_trait::async_trait;
use std::sync::Arc;

pub struct MatchReportSpaceOwnership {
    repo: Arc<dyn IMatchReportRepository>,
}

impl MatchReportSpaceOwnership {
    pub fn new(repo: Arc<dyn IMatchReportRepository>) -> Self {
        Self { repo }
    }
}

#[async_trait]
impl ISpaceOwnership for MatchReportSpaceOwnership {
    fn param(&self) -> &'static str {
        "match_report_id"
    }

    async fn space_of(&self, id: &str) -> Option<SpaceId> {
        match self.repo.find_space_id(id).await {
            Ok(Some(brut)) => SpaceId::try_new(&brut).ok(),
            Ok(None) => None,
            Err(e) => {
                tracing::error!("space_ownership match_report {id} : {e:?}");
                None
            }
        }
    }
}
