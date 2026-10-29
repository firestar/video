//! Storage crate: R2/S3 client wrapper.
//!
//! Wraps `aws-sdk-s3`: presign PUT (upload URL), signed GET (download URL),
//! direct upload, direct download, delete, exists.

use std::time::Duration;

use aws_config::{BehaviorVersion, SdkConfig};
use aws_credential_types::Credentials;
use aws_sdk_s3::presigning::PresigningConfig;
use aws_sdk_s3::types::ChecksumAlgorithm;
use aws_sdk_s3::Client;
use aws_smithy_types::byte_stream::ByteStream;
use bytes::Bytes;
use thiserror::Error;

/// Errors from storage operations.
#[derive(Debug, Error)]
pub enum StorageError {
    #[error("aws sdk error: {0}")]
    Aws(String),
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("config error: {0}")]
    Config(String),
}

/// High-level S3/R2 storage client.
#[derive(Debug, Clone)]
pub struct Storage {
    client: Client,
    bucket: String,
    base_url: String,
}

impl Storage {
    /// Build a client from explicit config (async — resolves AWS config).
    #[allow(clippy::too_many_arguments)]
    pub async fn new(
        region: &str,
        endpoint: Option<&str>,
        bucket: &str,
        access_key: &str,
        secret_key: &str,
        base_url: Option<&str>,
    ) -> Result<Self, StorageError> {
        let mut loader = aws_config::defaults(BehaviorVersion::latest())
            .region(aws_types::region::Region::new(region.to_string()))
            .credentials_provider(Credentials::new(
                access_key,
                secret_key,
                None,
                None,
                "static",
            ));
        if let Some(ep) = endpoint.filter(|e| !e.is_empty()) {
            loader = loader.endpoint_url(ep);
        }
        let aws_config: SdkConfig = loader.load().await;
        let client = Client::from_conf(aws_sdk_s3::config::Builder::from(&aws_config).force_path_style(true).build());
        Ok(Self {
            client,
            bucket: bucket.to_string(),
            base_url: base_url.unwrap_or_default().trim_end_matches('/').to_string(),
        })
    }

    /// Generate a presigned PUT URL for direct browser upload.
    pub async fn presign_put(
        &self,
        key: &str,
        ttl: Duration,
        content_type: Option<&str>,
    ) -> Result<String, StorageError> {
        let mut req = self
            .client
            .put_object()
            .bucket(&self.bucket)
            .key(key);
        if let Some(ct) = content_type {
            req = req.content_type(ct);
        }
        let config =
            PresigningConfig::expires_in(ttl).map_err(|e| StorageError::Config(e.to_string()))?;
        let signed = req
            .presigned(config)
            .await
            .map_err(|e| StorageError::Aws(e.to_string()))?;
        Ok(signed.uri().to_string())
    }

    /// Generate a signed GET URL for download/streaming.
    pub async fn signed_get(&self, key: &str, ttl: Duration) -> Result<String, StorageError> {
        let req = self
            .client
            .get_object()
            .bucket(&self.bucket)
            .key(key);
        let config =
            PresigningConfig::expires_in(ttl).map_err(|e| StorageError::Config(e.to_string()))?;
        let signed = req
            .presigned(config)
            .await
            .map_err(|e| StorageError::Aws(e.to_string()))?;
        Ok(signed.uri().to_string())
    }

    /// Upload bytes directly to the bucket.
    pub async fn upload(
        &self,
        key: &str,
        data: Bytes,
        content_type: &str,
    ) -> Result<(), StorageError> {
        let body: ByteStream = data.into();
        self.client
            .put_object()
            .bucket(&self.bucket)
            .key(key)
            .body(body)
            .content_type(content_type)
            .checksum_algorithm(ChecksumAlgorithm::Sha256)
            .send()
            .await
            .map_err(|e| StorageError::Aws(e.to_string()))?;
        Ok(())
    }

    /// Download object bytes.
    pub async fn download(&self, key: &str) -> Result<Bytes, StorageError> {
        let out = self
            .client
            .get_object()
            .bucket(&self.bucket)
            .key(key)
            .send()
            .await
            .map_err(|e| StorageError::Aws(e.to_string()))?;
        let aggregated = out.body.collect().await.map_err(|e| StorageError::Io(e.into()))?;
        Ok(aggregated.into_bytes())
    }

    /// Delete an object.
    pub async fn delete(&self, key: &str) -> Result<(), StorageError> {
        self.client
            .delete_object()
            .bucket(&self.bucket)
            .key(key)
            .send()
            .await
            .map_err(|e| StorageError::Aws(e.to_string()))?;
        Ok(())
    }

    /// Check object existence.
    pub async fn exists(&self, key: &str) -> Result<bool, StorageError> {
        match self
            .client
            .head_object()
            .bucket(&self.bucket)
            .key(key)
            .send()
            .await
        {
            Ok(_) => Ok(true),
            Err(e) => {
                let s = e.to_string();
                if s.contains("NotFound") || s.contains("404") || s.contains("NoSuchKey") {
                    Ok(false)
                } else {
                    Err(StorageError::Aws(s))
                }
            }
        }
    }

    /// Build a public URL for a key (if base_url is configured).
    pub fn public_url(&self, key: &str) -> Option<String> {
        if self.base_url.is_empty() {
            None
        } else {
            Some(format!("{}/{}", self.base_url, key))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_storage(base_url: &str) -> Storage {
        Storage {
            client: Client::from_conf(
                (&SdkConfig::builder()
                    .behavior_version(BehaviorVersion::latest())
                    .build())
                    .into(),
            ),
            bucket: "b".into(),
            base_url: base_url.trim_end_matches('/').to_string(),
        }
    }

    #[test]
    fn public_url_uses_base() {
        let s = make_storage("https://cdn.example.com/");
        assert_eq!(
            s.public_url("a/b.mp4"),
            Some("https://cdn.example.com/a/b.mp4".into())
        );
    }

    #[test]
    fn public_url_none_without_base() {
        let s = make_storage("");
        assert_eq!(s.public_url("a/b.mp4"), None);
    }
}
