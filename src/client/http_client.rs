use std::collections::HashMap;

type GenericError = Box<dyn std::error::Error + Send + Sync>;
type Result<T> = std::result::Result<T, GenericError>;


pub fn http_request(request_type: &str, endpoint: &str, body: Option<String>) -> String {
    let req_no_body = format!(
        "{} {} HTTP/1.1\r\nContent-Type: application/json\r\n",
        request_type, endpoint
    );
    if body.is_some() {
        let length = body.as_ref().unwrap().len();
        return format!(
            "{}Content-Length: {}\r\n\r\n{}",
            req_no_body,
            // body.clone().unwrap().len(),
            length,
            body.unwrap()
        );
    }
    format!("{}\r\n", req_no_body,)
}

fn process_res_line(s: &str) -> (&str, &str) {
    let mut words = s.split_whitespace();
    let version = words.next().unwrap();
    let status_code = words.next().unwrap();
    (version, status_code)
}

fn process_header_line(s: &str) -> (&str, &str) {
    let mut header_items = s.split(": ");
    let mut key = "";
    let mut value = "";
    if let Some(k) = header_items.next() {
        key = k;
    }
    if let Some(v) = header_items.next() {
        value = v;
    }
    (key, value)
}

pub fn res_into_parts(res: String) -> (String, String, String) {
    // let mut parsed_version = "";
    let mut parsed_status_code = "";
    let mut parsed_headers: HashMap<&str, &str> = HashMap::new();
    let mut parsed_msg_body = "";
    for line in res.lines() {
        if line.contains("HTTP") {
            let (_version, status_code) = process_res_line(line);
            // parsed_version = version;
            parsed_status_code = status_code;
        } else if line.contains(": ") { // 注意这里多一个空格, 否则就会把json格式数据也放到headers里面
            let (key, value) = process_header_line(line);
            parsed_headers.insert(key, value);
        } else if line.len() == 0 {
        } else {
            parsed_msg_body = line;
        }
    }

    let parsed_status_text: String = match parsed_status_code {
        "200" => "OK".into(),
        "204" => "No Content".into(),
        "400" => "Bad Request".into(),
        "404" => "Not Found".into(),
        "500" => "Internal Server Error".into(),
        _ => "Not Found".into(),
    };

    (parsed_status_code.to_string(), parsed_status_text, parsed_msg_body.to_string())
}

pub struct HttpUnixClient {

}

#[cfg(test)]
mod http_client_test {
    use std::error::Error;
    use hyper::{body::HttpBody, Client};
    use hyperlocal::{UnixClientExt, Uri};
    use tokio::io::{self, AsyncWriteExt as _};

    type GenericError = Box<dyn std::error::Error + Send + Sync>;
    type Result<T> = std::result::Result<T, GenericError>;

    async fn create_client() -> Result<()> {
        let url = Uri::new("/tmp/hyperlocal.sock", "/").into();

        let client = Client::unix();

        let mut response = client.get(url).await?;

        while let Some(next) = response.data().await {
            let chunk = next?;
            io::stdout().write_all(&chunk).await?;
        }

        Ok(())
    }

    #[test]
    fn test_create_client() {
        tokio::task::spawn(create_client());
    }
}