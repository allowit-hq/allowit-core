//! Reputation: what owners report about the services that their agents paid, as a reader sees it.
//!
//! An owner votes `good` or `bad` on one of three separate signals of one settled request:
//! `suitability` (the service suited the task), `delivery` (the seller delivered what the payment
//! bought) and `quality` (the output was good). A label reads the votes on one subject for one
//! reader and keeps three populations apart: this policy, the owner's other policies and other
//! owners. A label without support is unknown. Labels are heuristic signals, not probabilities.
//! The server keeps the votes and computes the labels; these are the values that it returns.
use serde::{Deserialize, Serialize};

/// One of the three separate signals.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Signal {
    Suitability,
    Delivery,
    Quality,
}
impl Signal {
    pub const ALL: [Self; 3] = [Self::Suitability, Self::Delivery, Self::Quality];
}

/// An owner's vote on one signal.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Vote {
    Good,
    Bad,
}

/// One label: its support and, with support, a good and a bad score from 0 to 100. `support` 0
/// is unknown and carries no score.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Label {
    pub support: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub good: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bad: Option<u32>,
}
impl Label {
    pub fn is_unknown(&self) -> bool {
        self.support == 0
    }
}

/// The labels of one signal for the populations that the reader may see. A public reader gets
/// `others` only.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Populations {
    /// Earlier requests of this policy.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub policy: Option<Label>,
    /// This owner's other policies.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub owner: Option<Label>,
    /// Other owners.
    pub others: Label,
}

/// The labels of one subject: three separate signals.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Labels {
    pub suitability: Populations,
    pub delivery: Populations,
    pub quality: Populations,
}
impl Labels {
    pub fn signal(&self, signal: Signal) -> &Populations {
        match signal {
            Signal::Suitability => &self.suitability,
            Signal::Delivery => &self.delivery,
            Signal::Quality => &self.quality,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn labels_keep_three_signals_and_unknown_reads_as_support_zero() {
        let labels: Labels = serde_json::from_value(json!({
            "suitability": {"others": {"support": 0}},
            "delivery": {"policy": {"support": 1, "good": 0, "bad": 33}, "owner": {"support": 0},
                "others": {"support": 5, "good": 55, "bad": 10}},
            "quality": {"others": {"support": 0}}
        }))
        .unwrap();
        assert!(labels.signal(Signal::Suitability).others.is_unknown());
        assert_eq!(labels.delivery.policy.unwrap().bad, Some(33));
        assert_eq!(
            serde_json::to_value(Label::default()).unwrap(),
            json!({"support": 0})
        );
        assert!(
            serde_json::from_value::<Labels>(json!({"suitability": {"others": {"support": 0}},
            "delivery": {"others": {"support": 0}}, "quality": {"others": {"support": 0}},
            "price": {"others": {"support": 0}}}))
            .is_err()
        );
        assert_eq!(serde_json::to_value(Vote::Bad).unwrap(), "bad");
        assert_eq!(serde_json::to_value(Signal::Delivery).unwrap(), "delivery");
    }
}
