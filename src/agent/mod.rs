pub mod agent;

use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct HttpRequest {
    pub(crate) method: String,
    pub(crate) path: String,
    pub(crate) version: String,
    pub(crate) headers: HashMap<String, String>,
    pub(crate) body: Option<String>,
}

impl HttpRequest {
    pub fn new(
        method: &str,
        path: &str,
        content_length: Option<usize>,
        body: Option<String>,
    ) -> Self {
        let mut headers = HashMap::new();
        if let Some(content_length) = content_length {
            headers.insert("Content-Length".to_string(), content_length.to_string());
        }
        HttpRequest {
            method: method.into(),
            path: path.into(),
            version: "HTTP/1.1".into(),
            headers,
            body,
        }
    }
}

// 序列化 HTTP 请求
pub fn serialize_request(request: &HttpRequest) -> String {
    // method uri version
    let mut request_str = format!(
        "{} {} {}\r\n",
        request.method, request.path, request.version
    );

    // add headers
    for (key, value) in &request.headers {
        request_str.push_str(&format!("{}: {}\r\n", key, value));
    }

    // add `Content-Length` header
    if let Some(body) = &request.body {
        request_str.push_str(&format!("Content-Length: {}\r\n", body.len()));
    }

    // empty line splitting headers and body
    request_str.push_str("\r\n");

    // add body
    if let Some(body) = &request.body {
        request_str.push_str(body);
    }

    request_str
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_request_without_body() {
        // Create an example HTTP request
        let mut request_headers = HashMap::new();
        request_headers.insert("Host".to_string(), "example.com".to_string());
        request_headers.insert("Connection".to_string(), "keep-alive".to_string());

        let request = HttpRequest {
            method: "GET".to_string(),
            path: "/".to_string(),
            version: "HTTP/1.1".to_string(),
            headers: request_headers,
            body: None,
        };

        // Serialize the request
        let serialized_request = serialize_request(&request);
        let expected_request =
            "GET / HTTP/1.1\r\nHost: example.com\r\nConnection: keep-alive\r\n\r\n".to_string();

        let expected_request_anothor_possibility =
            "GET / HTTP/1.1\r\nConnection: keep-alive\r\nHost: example.com\r\n\r\n".to_string();

        assert!(
            (serialized_request == expected_request)
                || (serialized_request == expected_request_anothor_possibility)
        );

        let mut headers = [httparse::EMPTY_HEADER; 64];
        let mut req = httparse::Request::new(&mut headers);
        let result = req.parse(serialized_request.as_bytes()).unwrap();
        assert!(result.is_complete());
    }

    #[test]
    fn test_request_with_body() {
        // Create an example HTTP request
        let mut request_headers = HashMap::new();
        request_headers.insert("Host".to_string(), "example.com".to_string());
        request_headers.insert("Connection".to_string(), "keep-alive".to_string());
        let body = "this is body".to_string();

        let request = HttpRequest {
            method: "GET".to_string(),
            path: "/".to_string(),
            version: "HTTP/1.1".to_string(),
            headers: request_headers,
            body: Some(body),
        };

        // Serialize the request
        let serialized_request = serialize_request(&request);
        let expected_request =
            "GET / HTTP/1.1\r\nHost: example.com\r\nConnection: keep-alive\r\nContent-Length: 12\r\n\r\nthis is body".to_string();
        let expected_request_anothor_possibility =
            "GET / HTTP/1.1\r\nConnection: keep-alive\r\nHost: example.com\r\nContent-Length: 12\r\n\r\nthis is body".to_string();

        assert!(
            (serialized_request == expected_request)
                || (serialized_request == expected_request_anothor_possibility)
        );

        let mut headers = [httparse::EMPTY_HEADER; 64];
        let mut req = httparse::Request::new(&mut headers);
        let result = req.parse(serialized_request.as_bytes()).unwrap();
        assert!(result.is_complete());
        assert_eq!(result.unwrap(), 81);
    }
}
