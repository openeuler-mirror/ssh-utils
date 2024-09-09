use crate::ssh::ssh_session::{SshSession, AuthMethod};
use std::sync::Arc;
use anyhow::Result;
use async_trait::async_trait;
use russh::keys::*;
use russh::*;
use tokio::net::ToSocketAddrs;
use super::common::{default_ssh_config, SshChannel};

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

pub struct PasswordSession {
    session: client::Handle<Client>
}

#[async_trait]
impl SshSession for PasswordSession {
    async fn connect<A: ToSocketAddrs + Send>(
        user: impl Into<String> + Send,
        auth: impl Into<AuthMethod> + Send,
        addrs: A,
    ) -> Result<Self> {
        let config = default_ssh_config();
        let config = Arc::new(config);
        let sh = Client {};

        let mut session = client::connect(config, addrs, sh).await?;

        let user = user.into();
        let auth = auth.into();

        match auth {
            AuthMethod::Password(password) => {
                let auth_res = session.authenticate_password(user, password).await?;
                if !auth_res {
                    anyhow::bail!("Authentication (with password) failed");
                }
            }
            AuthMethod::Key(_) => {
                anyhow::bail!("Key authentication not implemented for PasswordSession");
            }
        }
        Ok(Self { 
            session
        })
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