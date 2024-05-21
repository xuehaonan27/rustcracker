use crate::{
    micro_http::{Http, HttpMethod},
    ser::Serde,
};

pub struct Command {
    pub(crate) method: HttpMethod,
    pub(crate) url: String,
    pub(crate) data: Box<dyn Serde>,
}

impl Http for Command {
    fn encode(&self) -> String {
        let s = self.data.encode();
        format!(
            "{} {} HTTP/1.1\r\nContent-Length: {}\r\nContent-Type: application/json\r\nAccept: application/json\r\n\r\n{}",
            self.method.as_str(),
            self.url,
            s.len(),
            s
        )
    }
}
