use crate::{
    micro_http::{Http, HttpMethod},
    ser::Serde, RtckResult,
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

    fn decode<S: AsRef<str>>(line: &S) -> Self {
        let mut iter = line.as_ref().lines();
        let mut first_line = iter.next().expect("Should not fail").split_ascii_whitespace().skip(1);

        let code = first_line.next().expect("Should be code");
        let desc = first_line.next().expect("Should be desc");

        

        for line in line.as_ref().lines() {

        }

        todo!()
    }
}
