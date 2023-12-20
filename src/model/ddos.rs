#[derive(Debug, Clone)]
pub struct DDOS {
    pub url: String,
    pub workers_num: i32
}

impl DDOS {
    pub fn new(url: String, workers_num: i32) -> Self {
        return DDOS{url, workers_num}
    }
}
