use crate::community_advisors::models::{
    AdvisorReviewId, AdvisorReviewRow,
    ReviewRanking::{self, *},
};
use crate::rewards::Rewards;
use itertools::Itertools;
use std::collections::{BTreeMap, HashMap};

use serde::{Deserialize, Serialize};

pub type VeteranAdvisorId = String;

#[derive(Serialize)]
pub struct VeteranAdvisorIncentive {
    pub rewards: Rewards,
    pub reputation: u64,
}

pub type EligibilityThresholds = std::ops::RangeInclusive<usize>;

// TODO: for the sake of clarity, introduce a different naming between ca reviews and vca ranking

// Supposing to have a file with all the rankings for each review
// e.g. something like an expanded version of a AdvisorReviewRow
// [proposal_id, advisor, ratings, ..(other fields from AdvisorReviewRow).., ranking (good/excellent/filtered out), vca]
#[derive(Deserialize)]
pub struct VeteranRankingRow {
    #[serde(flatten)]
    advisor_review_row: AdvisorReviewRow,
    vca: VeteranAdvisorId,
}

impl VeteranRankingRow {
    fn score(&self) -> ReviewRanking {
        self.advisor_review_row.score()
    }

    fn review_id(&self) -> AdvisorReviewId {
        self.advisor_review_row.id()
    }
}

fn calc_final_ranking_per_review(rankings: &[&VeteranRankingRow]) -> ReviewRanking {
    let rankings_majority = rankings.len() / 2;
    let ranks = rankings.iter().counts_by(|r| r.score());

    match (ranks.get(&FilteredOut), ranks.get(&Excellent)) {
        (Some(filtered_out), _) if filtered_out > &rankings_majority => ReviewRanking::FilteredOut,
        (_, Some(excellent)) if excellent > &rankings_majority => ReviewRanking::Excellent,
        _ => ReviewRanking::Good,
    }
}

fn clamp_eligible_rewards(
    rankings: HashMap<VeteranAdvisorId, usize>,
    thresholds: EligibilityThresholds,
) -> BTreeMap<VeteranAdvisorId, usize> {
    rankings
        .into_iter()
        .filter_map(|(vca, n_rankings)| {
            if n_rankings < *thresholds.start() {
                return None;
            }

            Some((vca, n_rankings.min(*thresholds.end())))
        })
        .collect()
}

pub fn calculate_veteran_advisors_incentives(
    veteran_rankings: &[VeteranRankingRow],
    total_rewards: Rewards,
    rewards_thresholds: EligibilityThresholds,
    reputation_thresholds: EligibilityThresholds,
) -> HashMap<VeteranAdvisorId, VeteranAdvisorIncentive> {
    let final_rankings_per_review = veteran_rankings
        .iter()
        .into_group_map_by(|ranking| ranking.review_id())
        .into_iter()
        .map(|(review, rankings)| (review, calc_final_ranking_per_review(&rankings)))
        .collect::<BTreeMap<_, _>>();

    let eligible_rankings_per_vca = veteran_rankings
        .iter()
        .filter(|ranking| {
            final_rankings_per_review
                .get(&ranking.review_id())
                .unwrap()
                .is_positive()
                == ranking.score().is_positive()
        })
        .counts_by(|ranking| ranking.vca.clone());

    let reputation_eligible_rankings =
        clamp_eligible_rewards(eligible_rankings_per_vca.clone(), reputation_thresholds);

    let rewards_eligible_rankings =
        clamp_eligible_rewards(eligible_rankings_per_vca, rewards_thresholds);

    let tot_rewards_eligible_rankings =
        Rewards::from(rewards_eligible_rankings.values().sum::<usize>());

    reputation_eligible_rankings
        .into_iter()
        .zip(rewards_eligible_rankings.into_iter())
        .map(|((vca, reputation), (_vca2, reward))| {
            assert_eq!(vca, _vca2); // the use BTreeMaps ensures iteration is consistent
            (
                vca,
                VeteranAdvisorIncentive {
                    reputation: reputation as u64,
                    rewards: total_rewards * Rewards::from(reward) / tot_rewards_eligible_rankings,
                },
            )
        })
        .collect()
}
