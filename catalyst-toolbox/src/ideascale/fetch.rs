use crate::ideascale::models::de::{Fund, Funnel, Proposal, Stage};

use once_cell::sync::Lazy;
use serde::de::DeserializeOwned;
use url::Url;

use std::collections::HashMap;
use std::convert::TryInto;

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error(transparent)]
    RequestError(#[from] reqwest::Error),

    #[error("Could not get value from json, missing attribute {attribute_name}")]
    MissingAttribute { attribute_name: &'static str },
}

pub type Scores = HashMap<u32, f32>;
pub type Sponsors = HashMap<String, String>;

static BASE_IDEASCALE_URL: Lazy<url::Url> = Lazy::new(|| {
    "https://cardano.ideascale.com/a/rest/v1/"
        .try_into()
        .unwrap()
});

static CLIENT: Lazy<reqwest::Client> = Lazy::new(reqwest::Client::new);

async fn request_data<T: DeserializeOwned>(api_token: String, url: Url) -> Result<T, Error> {
    CLIENT
        .get(url)
        .header("api_token", api_token)
        .send()
        .await?
        .json()
        .await
        .map_err(Error::RequestError)
}

pub async fn get_funds_data(api_token: String) -> Result<Vec<Fund>, Error> {
    request_data(
        api_token,
        BASE_IDEASCALE_URL.join("campaigns/groups").unwrap(),
    )
    .await
}

pub async fn get_stages(api_token: String) -> Result<Vec<Stage>, Error> {
    request_data(api_token, BASE_IDEASCALE_URL.join("stages").unwrap()).await
}

/// we test token by running lightweight query and observe response code
pub async fn is_token_valid(api_token: String) -> Result<bool, Error> {
    let url = BASE_IDEASCALE_URL.join("profile/avatars").unwrap();

    let response = CLIENT
        .get(url)
        .header("api_token", api_token)
        .send()
        .await?;

    Ok(response.status() == 200)
}

pub async fn get_proposals_data(
    challenge_id: u32,
    api_token: String,
) -> Result<Vec<Proposal>, Error> {
    request_data(
        api_token,
        BASE_IDEASCALE_URL
            // ideascale API have some pager system which is not easy to find in the documentation
            // https://a.ideascale.com/api-docs/index.html#/rest-api-controller-v-1/ideasByCampaignUsingGET_2
            // in this case we want all of them, easiest way is to max out the page size.
            .join(&format!("campaigns/{}/ideas/0/100000", challenge_id))
            .unwrap(),
    )
    .await
}

pub async fn get_funnels_data_for_fund(api_token: String) -> Result<Vec<Funnel>, Error> {
    let challenges: Vec<Funnel> =
        request_data(api_token, BASE_IDEASCALE_URL.join("funnels").unwrap()).await?;
    Ok(challenges)
}
