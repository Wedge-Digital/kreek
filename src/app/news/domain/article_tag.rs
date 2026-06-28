use serde::{Deserialize, Serialize};
use std::fmt;
use std::str::FromStr;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ArticleTag {
    #[serde(rename = "NEWS")]
    News,
    #[serde(rename = "MATCH_REPORT")]
    MatchReport,
    #[serde(rename = "ANALYSIS")]
    Analysis,
    #[serde(rename = "ITW")]
    Itw,
    #[serde(rename = "TUTORIAL")]
    Tutorial,
}

impl fmt::Display for ArticleTag {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            ArticleTag::News => "NEWS",
            ArticleTag::MatchReport => "MATCH_REPORT",
            ArticleTag::Analysis => "ANALYSIS",
            ArticleTag::Itw => "ITW",
            ArticleTag::Tutorial => "TUTORIAL",
        };
        write!(f, "{s}")
    }
}

impl FromStr for ArticleTag {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "NEWS" => Ok(ArticleTag::News),
            "MATCH_REPORT" => Ok(ArticleTag::MatchReport),
            "ANALYSIS" => Ok(ArticleTag::Analysis),
            "ITW" => Ok(ArticleTag::Itw),
            "TUTORIAL" => Ok(ArticleTag::Tutorial),
            _ => Err(()),
        }
    }
}
