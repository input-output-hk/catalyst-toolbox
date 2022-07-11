use color_eyre::eyre::Result;
use reqwest::{
    blocking::Client,
    header::{HeaderMap, HeaderValue},
    Url,
};
use serde::Deserialize;

use super::{HttpClient, HttpResponse};

const BASE_IDEASCALE_URL: &str = "https://cardano.ideascale.com/a/rest/v1/";

pub struct ReqwestClient {
    client: Client,
    base_url: Url,
}

impl ReqwestClient {
    pub fn new(api_key: Option<&str>) -> Self {
        let mut client = Client::builder();

        if let Some(api_key) = api_key {
            let mut headers = HeaderMap::new();
            let mut auth_value = HeaderValue::from_str(api_key).unwrap();
            auth_value.set_sensitive(true);
            headers.insert("api_token", auth_value);
            client = client.default_headers(headers);
        };

        let client = client.build().unwrap();

        Self {
            client,
            base_url: BASE_IDEASCALE_URL.try_into().unwrap(),
        }
    }
}

impl HttpClient for ReqwestClient {
    fn get<T>(&self, path: &str) -> Result<HttpResponse<T>>
    where
        T: for<'a> Deserialize<'a>,
    {
        let url = self.base_url.join(path.as_ref())?;
        let response = self.client.get(url).send()?;
        let status = response.status();

        Ok(HttpResponse {
            _marker: std::marker::PhantomData,
            status,
            body: response.text()?,
        })
    }
}
