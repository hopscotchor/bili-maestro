use std::{borrow::Cow, collections::HashMap};

use anyhow::{anyhow, Result};
use reqwest::{
    header::{HeaderMap, HeaderValue},
    Method, Request,
};
use serde_json::Value;
use url::form_urlencoded::Parse;

use crate::bili::{client, credential::Credential};

pub struct PreProcess<'a> {
    request: &'a mut Request,
    data: HashMap<&'a str, &'a str>,
    need_verify: bool,
    need_csrf: bool,
    need_wbi: bool,
    need_wbi2: bool,
    credential: Option<Credential<'a>>,
}

impl<'a> PreProcess<'a> {
    pub fn new(request: &'a mut Request) -> Self {
        Self {
            request,
            data: HashMap::new(),
            need_verify: false,
            need_csrf: false,
            need_wbi: false,
            need_wbi2: false,
            credential: Some(Credential::new()),
        }
    }

    pub fn pre_verify_params(&mut self) -> Result<()> {
        if self.need_verify && self.credential.is_none() {
            return Err(anyhow!("need verify,but credential is None"));
        }
        if self.request.method() != Method::GET && self.need_csrf {
            return Err(anyhow!("not get method,and no csrf"));
        }
        // jsonp
        if !self
            .request
            .url()
            .query_pairs()
            .find(|(k, v)| k == &Cow::Borrowed("jsonp") && v == &Cow::Borrowed("jsonp"))
            .is_none()
        {
            self.request.url_mut().set_query(Some("callback=callback"));
        };
        if self.need_wbi2 {
            wbi2(self.request.url_mut().query_pairs())
        };
        if self.need_wbi && !self.credential.is_none() {
            wbi(
                self.request.url_mut().query_pairs(),
                get_wbi_mixin_key(&mut self.credential),
            );
        }
        if self.need_csrf
            && self.need_csrf
            && vec![Method::POST, Method::DELETE, Method::PATCH].contains(&self.request.method())
        {
            let bili_jct = self
                .credential
                .as_ref()
                .ok_or(anyhow!("credential is None"))?
                .clone()
                .bili_jct
                .ok_or(anyhow!("bili_jct is None"))?;

            self.data.insert("csrf", bili_jct);
            self.data.insert("csrf_token", bili_jct);
        }
        Ok(())
    }

    pub fn pre_handle_cookies(&mut self) -> Result<()> {
        // let mut cookies = self
        //     .credential
        //     .clone()
        //     .ok_or(anyhow!("credential is None"))?
        //     .get_cookie();
        // if cookies.get("buvid3").is_none() && cookies.get("buvid4").is_none() {}

        Ok(())
    }

    pub fn preprocess(&mut self) -> Result<()> {
        Ok(())
    }
}

pub fn get_wbi_mixin_key(credential: &mut Option<Credential>) -> String {
    todo!()
}
pub fn wbi2(params: Parse<'_>) {
    todo!()
}

pub fn wbi(params: Parse<'_>, mixin_key: String) {
    todo!()
}

pub async fn get_buvid() -> Result<HashMap<String, String>> {
    let url = "https://api.bilibili.com/x/frontend/finger/spi";
    let mut headers = HeaderMap::new();
    headers.insert("User-Agent", "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/131.0.0.0 Safari/537.36 Edg/131.0.0.0".parse()?);
    headers.insert("Referer", "https://www.bilibili.com".parse()?);
    let res = client::BiliClient::new()
        .http_client
        .request(Method::GET, url)
        .headers(headers)
        .send()
        .await?;
    let v: serde_json::Value = serde_json::from_str(&res.text().await?)?;
    let v = v
        .get("data")
        .ok_or(anyhow!("can not parse response body string"))?;
    let v = v
        .as_object()
        .ok_or(anyhow!("can not parse response body string"))?
        .clone();
    let mut data: HashMap<String, String> = HashMap::new();
    v.iter().for_each(|(k, v)| {
        if let Value::String(s) = v {
            data.insert((*k).clone(), (*s).clone());
        }
    });
    Ok(data)
}
