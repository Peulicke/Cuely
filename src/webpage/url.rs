// Cuely is an open source web search engine.
// Copyright (C) 2022 Cuely ApS
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as
// published by the Free Software Foundation, either version 3 of the
// License, or (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program.  If not, see <https://www.gnu.org/licenses/>.

use std::{fmt::Display, time::Duration};

use tracing::debug;

#[derive(Debug, PartialEq, Eq, Clone, Hash)]
pub struct Url(String);

impl Display for Url {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", &self.0)
    }
}

impl From<String> for Url {
    fn from(url: String) -> Self {
        Url(url)
    }
}

impl Url {
    pub fn strip_protocol(&self) -> &str {
        let mut start_host = 0;
        let url = &self.0;
        if url.starts_with("http://") || url.starts_with("https://") || url.starts_with("//") {
            start_host = url
                .find('/')
                .expect("It was checked that url starts with protocol");
            start_host += 2; // skip the two '/'
        }

        &url[start_host..]
    }

    pub fn strip_query(&self) -> &str {
        let url = &self.0;
        let mut start_query = url.len();
        if url.contains('?') {
            start_query = url.find('?').expect("The url contains atleast 1 '?'");
        }

        &url[..start_query]
    }

    pub fn host(&self) -> &str {
        let url = self.strip_protocol();

        let mut end_host = url.len();
        if url.contains('/') {
            end_host = url.find('/').expect("The url contains atleast 1 '/'");
        }

        &url[..end_host]
    }

    pub fn domain(&self) -> &str {
        let host = self.host();
        let num_punctuations: usize = host.chars().map(|c| if c == '.' { 1 } else { 0 }).sum();
        if num_punctuations > 1 {
            let domain_index = host.rfind('.').unwrap();
            let mut start_index = host[..domain_index].rfind('.').unwrap() + 1;

            if &host[start_index..] == "co.uk" {
                if let Some(new_start_index) = host[..start_index - 1].rfind('.') {
                    start_index = new_start_index + 1;
                } else {
                    start_index = 0;
                }
            }

            &host[start_index..]
        } else {
            host
        }
    }

    pub fn subdomain(&self) -> Option<&str> {
        if let Some(subdomain) = self.host().strip_suffix(self.domain()) {
            if subdomain.is_empty() || subdomain == "." {
                None
            } else {
                Some(&subdomain[..subdomain.len() - 1])
            }
        } else {
            None
        }
    }

    pub fn is_homepage(&self) -> bool {
        let url = self.strip_protocol();
        match url.find('/') {
            Some(idx) => idx == url.len() - 1,
            None => true,
        }
    }

    fn find_protocol_end(&self) -> usize {
        let mut start_host = 0;
        let url = &self.0;
        if url.starts_with("http://") || url.starts_with("https://") {
            start_host = url
                .find(':')
                .expect("It was checked that url starts with protocol");
        } else if url.starts_with("//") {
            start_host = url
                .find('/')
                .expect("It was checked that url starts with protocol")
                + 1;
        }
        start_host
    }
    pub fn protocol(&self) -> &str {
        &self.0[..self.find_protocol_end()]
    }

    pub fn site(&self) -> &str {
        let start = self.find_protocol_end() + 3;
        let url = &self.0[start..];

        let mut end_host = url.len();
        if url.contains('/') {
            end_host = url.find('/').expect("The url contains atleast 1 '/'");
        }

        &self.0[..end_host + start]
    }

    pub fn is_full_path(&self) -> bool {
        matches!(self.protocol(), "http" | "https" | "pdf")
    }

    pub fn prefix_with(&mut self, url: &Url) {
        self.0 = match (url.0.ends_with('/'), self.0.starts_with('/')) {
            (true, true) => url.site().to_string() + &self.0,
            (true, false) => url.0.clone() + &self.0,
            (false, true) => url.site().to_string() + &self.0,
            (false, false) => url.0.clone() + "/" + &self.0,
        };
    }

    pub fn full(&self) -> String {
        if self.find_protocol_end() == 0 {
            "https://".to_string() + &self.0
        } else {
            self.0.clone()
        }
    }

    pub async fn download_bytes(&self, timeout: Duration) -> Option<Vec<u8>> {
        let client = reqwest::Client::builder().timeout(timeout).build().unwrap();

        debug!("downloading {:?}", self.full());

        match client.get(self.full()).send().await {
            Ok(res) => {
                let bytes = res.bytes().await.ok()?.to_vec();
                Some(bytes)
            }
            Err(_) => None,
        }
    }

    pub fn raw(&self) -> &str {
        &self.0
    }

    pub fn is_valid_uri(&self) -> bool {
        self.full().as_str().parse::<http::Uri>().is_ok()
    }

    pub(crate) fn host_without_specific_subdomains(&self) -> &str {
        if let Some(subdomain) = self.subdomain() {
            if subdomain == "www" {
                self.domain()
            } else {
                self.host()
            }
        } else {
            self.host()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn double_slash_start() {
        let url: Url = "//scripts.dailymail.co.uk".to_string().into();

        assert_eq!(url.domain(), "dailymail.co.uk");
        assert_eq!(url.host(), "scripts.dailymail.co.uk");
    }

    #[test]
    fn co_uk_edgecase() {
        let url: Url = "dailymail.co.uk".to_string().into();

        assert_eq!(url.domain(), "dailymail.co.uk");
        assert_eq!(url.host(), "dailymail.co.uk");
        assert_eq!(url.full().as_str(), "https://dailymail.co.uk");
    }

    #[test]
    fn full() {
        let url: Url = "https://example.com".to_string().into();
        assert_eq!(url.full().as_str(), "https://example.com");

        let url: Url = "http://example.com".to_string().into();
        assert_eq!(url.full().as_str(), "http://example.com");
    }

    #[test]
    fn prefix_with() {
        let mut a: Url = "/test".to_string().into();
        let b: Url = "https://example.com".to_string().into();
        a.prefix_with(&b);
        assert_eq!(a.full().as_str(), "https://example.com/test");

        let mut a: Url = "test".to_string().into();
        let b: Url = "https://example.com".to_string().into();
        a.prefix_with(&b);
        assert_eq!(a.full().as_str(), "https://example.com/test");

        let mut a: Url = "test".to_string().into();
        let b: Url = "https://example.com/".to_string().into();
        a.prefix_with(&b);
        assert_eq!(a.full().as_str(), "https://example.com/test");

        let mut a: Url = "/test".to_string().into();
        let b: Url = "https://example.com/".to_string().into();
        a.prefix_with(&b);
        assert_eq!(a.full().as_str(), "https://example.com/test");
    }

    #[test]
    fn is_full_path() {
        let url: Url = "https://dailymail.co.uk".to_string().into();
        assert!(url.is_full_path());
    }

    #[test]
    fn is_valid() {
        let url: Url = "https://dailymail.co.uk".to_string().into();
        assert!(url.is_valid_uri());

        let url: Url = "da<>ilymail.co.uk".to_string().into();
        assert!(!url.is_valid_uri());
    }

    #[test]
    fn subdomain() {
        let url: Url = "https://test.example.com".to_string().into();
        assert_eq!(url.subdomain(), Some("test"));

        let url: Url = "https://test1.test2.example.com".to_string().into();
        assert_eq!(url.subdomain(), Some("test1.test2"));

        let url: Url = "https://example.com".to_string().into();
        assert_eq!(url.subdomain(), None);
    }
}
