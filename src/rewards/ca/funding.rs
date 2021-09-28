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
}
