use rust_decimal::Decimal;
use serde::Deserialize;

pub type Funds = Decimal;

#[derive(Deserialize)]
pub struct FundSetting {
    pub proposal_ratio: u8,
    pub bonus_ratio: u8,
    pub total: Funds,
}

impl FundSetting {
    #[inline]
    pub fn proposal_funds(&self) -> Funds {
        self.total * (Funds::from(self.proposal_ratio) / Funds::from(100))
    }

    #[inline]
    pub fn bonus_funds(&self) -> Funds {
        self.total * (Funds::from(self.bonus_ratio) / Funds::from(100))
    }

    #[inline]
    pub fn total_funds(&self) -> Funds {
        self.total
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
