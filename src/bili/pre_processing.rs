use std::borrow::Cow;

use anyhow::{anyhow, Result};
use reqwest::{Method, Request};
use url::form_urlencoded::Parse;

use crate::bili::credential::Credential;

pub struct PreProcess<'a> {
    request: &'a mut Request,
    need_verify: bool,
    need_csrf: bool,
    need_wbi: bool,
    need_wbi2: bool,
    credential: Option<Credential>,
}

impl<'a> PreProcess<'a> {
    pub fn new(request: &'a mut Request) -> Self {
        Self {
            request,
            need_verify: false,
            need_csrf: false,
            need_wbi: false,
            need_wbi2: false,
            credential: Some(Credential::new()),
        }
    }

    pub fn preprocess(&mut self) -> Result<()> {
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
            todo!("add body field")
        }

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
