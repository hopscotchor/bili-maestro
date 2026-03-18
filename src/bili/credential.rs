pub struct Credential {
    sessdata: Option<String>,
    bili_jct: Option<String>,
    buvid3: Option<String>,
    buvid4: Option<String>,
    dedeuserid: Option<String>,
    ac_time_value: Option<String>,
    proxy: Option<String>,
}

impl Credential {
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
}
