use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ParagraphType {
    #[serde(rename = "1_COLUMN")]
    OneColumn,
}
