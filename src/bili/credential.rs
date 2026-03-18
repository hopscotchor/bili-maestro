use std::collections::HashMap;

#[derive(Clone)]
pub struct Credential<'a> {
    pub sessdata: Option<&'a str>,
    pub bili_jct: Option<&'a str>,
    pub buvid3: Option<&'a str>,
    pub buvid4: Option<&'a str>,
    pub dedeuserid: Option<&'a str>,
    pub ac_time_value: Option<&'a str>,
    pub proxy: Option<&'a str>,
}

impl<'a> Credential<'a> {
    pub fn new() -> Self {
        Self {
            sessdata: None,
            bili_jct: None,
            buvid3: None,
            buvid4: None,
            dedeuserid: None,
            ac_time_value: None,
            proxy: None,
        }
    }

    pub fn get_cookie(&self) -> HashMap<&str, &str> {
        let mut cookies: HashMap<&str, &str> = HashMap::new();
        cookies.insert("SESSDATA", self.sessdata.unwrap_or(""));
        cookies.insert("buvid3", self.buvid3.unwrap_or(""));
        cookies.insert("buvid4", self.buvid4.unwrap_or(""));
        cookies.insert("bili_jct", self.bili_jct.unwrap_or(""));
        cookies.insert("ac_time_value", self.ac_time_value.unwrap_or(""));
        if let Some(de) = self.dedeuserid {
            cookies.insert("DedeUserID", de);
        }
        cookies
    }
}
