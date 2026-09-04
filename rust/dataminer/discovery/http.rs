use anyhow::{Context, Result, bail};
use reqwest::StatusCode;
use reqwest::blocking::Client;
use reqwest::header::HeaderMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;
use std::time::Duration;

const USER_AGENT: &str = "ympatcher-dataminer/0.4 (+https://github.com/pyanexya/ympatcher)";

#[derive(Debug, Clone, Default)]
pub struct CancellationToken(Arc<AtomicBool>);

impl CancellationToken {
    pub fn cancel(&self) {
        self.0.store(true, Ordering::Release);
    }

    pub fn is_cancelled(&self) -> bool {
        self.0.load(Ordering::Acquire)
    }
}

#[derive(Debug, Clone)]
pub struct HttpResponse {
    pub status: StatusCode,
    pub bytes: Vec<u8>,
}

#[derive(Debug, Clone)]
pub struct SharedHttpClient {
    client: Client,
    max_attempts: usize,
}

impl SharedHttpClient {
    pub fn new() -> Result<Self> {
        let client = Client::builder()
            .connect_timeout(Duration::from_secs(8))
            .timeout(Duration::from_secs(20))
            .user_agent(USER_AGENT)
            .build()
            .context("cannot build dataminer HTTP client")?;
        Ok(Self {
            client,
            max_attempts: 3,
        })
    }

    pub fn get(
        &self,
        url: &str,
        headers: HeaderMap,
        cancellation: &CancellationToken,
    ) -> Result<HttpResponse> {
        let mut last_error = None;
        for attempt in 0..self.max_attempts {
            if cancellation.is_cancelled() {
                bail!("request cancelled");
            }
            match self.client.get(url).headers(headers.clone()).send() {
                Ok(response) => {
                    let status = response.status();
                    if should_retry_status(status) && attempt + 1 < self.max_attempts {
                        tracing::warn!(
                            url,
                            status = status.as_u16(),
                            attempt,
                            "temporary HTTP failure"
                        );
                        self.backoff(attempt, cancellation)?;
                        continue;
                    }
                    let bytes = response.bytes().context("cannot read HTTP response body")?;
                    return Ok(HttpResponse {
                        status,
                        bytes: bytes.to_vec(),
                    });
                }
                Err(error) => {
                    let retryable = error.is_connect() || error.is_timeout() || error.is_request();
                    last_error = Some(error);
                    if !retryable || attempt + 1 == self.max_attempts {
                        break;
                    }
                    tracing::warn!(url, attempt, "temporary network failure");
                    self.backoff(attempt, cancellation)?;
                }
            }
        }
        Err(last_error
            .map(anyhow::Error::from)
            .unwrap_or_else(|| anyhow::anyhow!("HTTP request failed")))
    }

    fn backoff(&self, attempt: usize, cancellation: &CancellationToken) -> Result<()> {
        let slices = 2_u32.pow(attempt as u32) * 5;
        for _ in 0..slices {
            if cancellation.is_cancelled() {
                bail!("request cancelled");
            }
            thread::sleep(Duration::from_millis(100));
        }
        Ok(())
    }
}

fn should_retry_status(status: StatusCode) -> bool {
    status == StatusCode::TOO_MANY_REQUESTS || status.is_server_error()
}
