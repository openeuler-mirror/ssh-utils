use crate::ssh::ssh_session::{SshSession, AuthMethod};
use std::convert::TryFrom;
use std::env;
use std::sync::Arc;

use anyhow::Result;
use async_trait::async_trait;
use crossterm::terminal::size;
use russh::keys::*;
use russh::*;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::ToSocketAddrs;

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
    session: client::Handle<Client>,
}

#[async_trait]
impl SshSession for PasswordSession {
    async fn connect<A: ToSocketAddrs + Send>(
        user: impl Into<String> + Send,
        auth: impl Into<AuthMethod> + Send,
        addrs: A,
    ) -> Result<Self> {
        let config = client::Config::default();
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

        Ok(Self { session })
    }

    async fn call(&mut self, command: &str) -> Result<u32> {
        let mut channel = self.session.channel_open_session().await?;

        // This example doesn't terminal resizing after the connection is established
        let (w, h) = size()?;

        // Request an interactive PTY from the server
        channel
            .request_pty(
                false,
                &env::var("TERM").unwrap_or("xterm".into()), // TERM=xterm 是一个环境变量设置，用于指定终端类型
                w as u32,
                h as u32,
                0,
                0,
                &[], // ideally you want to pass the actual terminal modes here
            )
            .await?;
        channel.exec(true, command).await?;

        let code;
        let mut stdin = tokio_fd::AsyncFd::try_from(0)?;
        let mut stdout = tokio_fd::AsyncFd::try_from(1)?;
        let mut buf = vec![0; 1024];
        let mut stdin_closed = false;

        loop {
            // Handle one of the possible events:
            tokio::select! {
                // There's terminal input available from the user
                r = stdin.read(&mut buf), if !stdin_closed => {
                    match r {
                        Ok(0) => { // 没有输入
                            stdin_closed = true;
                            channel.eof().await?;
                        },
                        // Send it to the server
                        Ok(n) => channel.data(&buf[..n]).await?, //发送数据
                        Err(e) => return Err(e.into()),
                    };
                },
                // There's an event available on the session channel
                Some(msg) = channel.wait() => {
                    match msg {
                        // Write data to the terminal
                        ChannelMsg::Data { ref data } => {
                            stdout.write_all(data).await?;
                            stdout.flush().await?;
                        }
                        // The command has returned an exit code
                        ChannelMsg::ExitStatus { exit_status } => {
                            code = exit_status;
                            if !stdin_closed {
                                channel.eof().await?;
                            }
                            break;
                        }
                        _ => {}
                    }
                },
            }
        }
        Ok(code)
    }

    async fn close(&mut self) -> Result<()> {
        self.session
            .disconnect(Disconnect::ByApplication, "", "English")
            .await?;
        Ok(())
    }
}

#[derive(clap::Parser)]
#[clap(trailing_var_arg = true)]
pub struct Cli {
    #[clap(index = 1)]
    host: String,

    #[clap(long, default_value_t = 22)]
    port: u16,

    #[clap(long, short)]
    username: String,

    #[clap(long, short)]
    password: String,

    #[clap(num_args = 1.., index = 2, required = true)]
    command: Vec<String>,
}