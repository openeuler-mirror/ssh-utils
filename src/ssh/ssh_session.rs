use std::path::PathBuf;
use anyhow::Result;
use tokio::net::ToSocketAddrs;

#[async_trait::async_trait]
pub trait SshSession {
    async fn connect<A: ToSocketAddrs + Send>(
        user: impl Into<String> + Send,
        auth: impl Into<AuthMethod> + Send,
        addrs: A,
    ) -> Result<Self>
    where
        Self: Sized;

    async fn call(&mut self, command: &str) -> Result<u32>;
    async fn close(&mut self) -> Result<()>;
}

pub enum AuthMethod {
    Password(String),
    Key(PathBuf),
}