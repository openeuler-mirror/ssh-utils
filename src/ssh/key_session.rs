use std::sync::Arc;
use anyhow::Result;
use async_trait::async_trait;
use russh::keys::*;
use russh::*;
use tokio::net::ToSocketAddrs;
use super::common::SshChannel;
use super::ssh_session::{AuthMethod, SshSession};

pub struct Client {}

// More SSH event handlers
// can be defined in this trait
// In this example, we're only using Channel, so these aren't needed.
#[async_trait]
impl client::Handler for Client {
    type Error = russh::Error;

    async fn check_server_key(
        &mut self,
        _server_public_key: &key::PublicKey,
    ) -> Result<bool, Self::Error> {
        Ok(true)
    }
}

/// This struct is a convenience wrapper
/// around a russh client
/// that handles the input/output event loop
pub struct KeySession {
    session: client::Handle<Client>,
}

#[async_trait::async_trait]
impl SshSession for KeySession {
    async fn connect<A: ToSocketAddrs + Send>(
        user: impl Into<String> + Send,
        auth: impl Into<AuthMethod> + Send,
        addrs: A,
    ) -> Result<Self> {
        let auth = auth.into();
        let key_pair = match auth {
            AuthMethod::Key(path) => path,
            AuthMethod::Password(_) => anyhow::bail!("KeySession only supports key authentication"),
        };

        //let key_pair: key::KeyPair = load_secret_key(key_path, None)?;

        let config = client::Config {
            //inactivity_timeout: Some(Duration::from_secs(5)),
            ..<_>::default()
        };

        let config = Arc::new(config);
        let sh = Client {};

        let mut session = client::connect(config, addrs, sh).await?;

        // 使用公钥进行认证
        let auth_res = session
            .authenticate_publickey(user, Arc::new(key_pair))
            .await?;

        if !auth_res {
            anyhow::bail!("public key authentication failed");
        }

        Ok(Self { session })
    }

    async fn call(&mut self, command: &str) -> Result<u32> {
        let channel = self.session.channel_open_session().await?;
        let mut ssh_channel = SshChannel::new(channel).await?;
        ssh_channel.call(command).await
    }

    async fn close(&mut self) -> Result<()> {
        self.session
            .disconnect(Disconnect::ByApplication, "", "English")
            .await?;
        Ok(())
    }
}