use fixed::types::U64F64;
use serde::Deserialize;

pub type Funds = U64F64;

#[derive(Deserialize)]
pub struct FundSetting {
    proposal_ratio: u8,
    bonus_ratio: u8,
    total: Funds,
}

impl FundSetting {
    #[inline]
    pub fn proposal_funds(self) -> Funds {
        self.total * (U64F64::from(self.proposal_ratio) / 100)
    }

    #[inline]
    pub fn bonus_funds(self) -> Funds {
        self.total * (U64F64::from(self.bonus_ratio) / 100)
    }

    #[inline]
    pub fn total_funds(&self) -> Funds {
        self.total
    }

    #[inline]
    pub fn funds_per_proposal(&self, number_of_proposals: u64) -> Funds {
        self.total / Funds::from(number_of_proposals)
    }
}

#[derive(Deserialize)]
pub struct ProposalRewardSlots {
    pub excellent_slots: u64,
    pub good_slots: u64,
    pub filled_slots: u64,
}

impl Default for ProposalRewardSlots {
    fn default() -> Self {
        Self {
            excellent_slots: 12,
            good_slots: 4,
            filled_slots: 36,
        }
    }
}
