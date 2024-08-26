use anyhow::Result;
use crossterm::terminal::size;
use russh::{client::Msg, *};
use std::convert::TryFrom;
use std::env;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

pub struct SshChannel {
    channel: Channel<Msg>,
    last_size: (u16, u16),
}

impl SshChannel {
    pub async fn new(channel: Channel<Msg>) -> Result<Self> {
        let (w, h) = size()?;
        Ok(Self {
            channel,
            last_size: (w, h),
        })
    }

    pub async fn call(&mut self, command: &str) -> Result<u32> {
        let (w, h) = self.last_size;

        // Request an interactive PTY from the server
        self.channel
            .request_pty(
                false,
                &env::var("TERM").unwrap_or("xterm".into()),
                w as u32,
                h as u32,
                0,
                0,
                &[],
            )
            .await?;
        self.channel.exec(true, command).await?;

        let code;
        let mut stdin = tokio_fd::AsyncFd::try_from(0)?;
        let mut stdout = tokio_fd::AsyncFd::try_from(1)?;
        let mut buf = vec![0; 1024];
        let mut stdin_closed = false;

        loop {
            tokio::select! {
                r = stdin.read(&mut buf), if !stdin_closed => {
                    match r {
                        Ok(0) => {
                            stdin_closed = true;
                            self.channel.eof().await?;
                        },
                        Ok(n) => self.channel.data(&buf[..n]).await?,
                        Err(e) => return Err(e.into()),
                    };
                },
                Some(msg) = self.channel.wait() => {
                    match msg {
                        ChannelMsg::Data { ref data } => {
                            let (w, h) = size()?;
                            if (w, h) != self.last_size {
                                self.channel.window_change(w as u32, h as u32, 0, 0).await?;
                                self.last_size = (w, h);
                            }
                            stdout.write_all(data).await?;
                            stdout.flush().await?;
                        }
                        ChannelMsg::ExitStatus { exit_status } => {
                            code = exit_status;
                            if !stdin_closed {
                                self.channel.eof().await?;
                            }
                            break;
                        }
                        _ => {}
                    }
                }
            }
        }
        Ok(code)
    }
}