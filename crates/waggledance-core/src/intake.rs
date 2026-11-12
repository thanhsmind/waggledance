use crate::error::{Error, Result};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IntakeVerdict {
    Dispatched,
    Held,
    Skipped,
    Covered,
}

impl IntakeVerdict {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Dispatched => "dispatched",
            Self::Held => "held",
            Self::Skipped => "skipped",
            Self::Covered => "covered",
        }
    }

    pub fn parse(s: &str) -> Result<Self> {
        match s {
            "dispatched" => Ok(Self::Dispatched),
            "held" => Ok(Self::Held),
            "skipped" => Ok(Self::Skipped),
            "covered" => Ok(Self::Covered),
            other => Err(Error::Other(format!("unknown intake verdict `{other}`"))),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct IntakeItem {
    pub key: String,
    pub source: String,
    pub item_id: String,
    pub title: String,
    pub url: String,
    pub project: Option<String>,
    pub confidence: f64,
    pub reason: String,
    pub verdict: IntakeVerdict,
    pub run_id: Option<String>,
    pub notified: bool,
    pub first_seen: String,
}
