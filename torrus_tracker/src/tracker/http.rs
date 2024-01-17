use anyhow::{Ok, Result};
use reqwest::{Client, StatusCode};
use std::{collections::HashMap, ops::Deref};
use url::Url;

use crate::{request::TrackerRequest, TrackerResponse};

pub struct HttpTracker {
    url: Url,
    client: Client,
}

impl HttpTracker {
    pub fn new(url: Url) -> Self {
        Self {
            url,
            client: Client::new(),
        }
    }

    pub async fn send_request(&mut self, request: TrackerRequest) -> Result<TrackerResponse> {
        let query = Self::request_to_hash_map(request);

        let resp = self
            .client
            .get(self.url.to_string())
            .query(&query)
            .send()
            .await?;

        if StatusCode::OK != resp.status() {
            anyhow::bail!("Error HTTP");
        }

        let bytes = resp.bytes().await?;
        let tracker_response = serde_bencode::de::from_bytes(&bytes)?;
        Ok(tracker_response)
    }

    fn request_to_hash_map(request: TrackerRequest) -> HashMap<String, String> {
        use url::form_urlencoded::byte_serialize;

        let mut query_map = HashMap::new();

        let mut append_pair = |key: String, val: String| {
            query_map.insert(key, val);
        };

        let info_hash_str: String = byte_serialize(request.info_hash.deref()).collect();
        let peer_id_str: String = byte_serialize(request.peer_id.deref()).collect();

        append_pair("info_hash".to_string(), info_hash_str);
        append_pair("peer_id".to_string(), peer_id_str);
        append_pair("downloaded".to_string(), request.downloaded.to_string());
        append_pair("left".to_string(), request.left.to_string());
        append_pair("uploaded".to_string(), request.uploaded.to_string());
        append_pair("event".to_string(), request.event);
        append_pair("ip_address".to_string(), request.ip_address.to_string());
        append_pair("key".to_string(), request.key.to_string());
        append_pair("num_want".to_string(), request.num_want.to_string());
        append_pair("port".to_string(), request.port.to_string());
        append_pair("no_peer_id".to_string(), "0".to_string());
        append_pair("compact".to_string(), "1".to_string());
        query_map
    }
}
