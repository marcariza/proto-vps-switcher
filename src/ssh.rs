use std::process::{Command, ExitStatus};

use anyhow::{Context, Result};

use crate::config::{AuthConfig, ConnectionConfig};

pub fn connect(connection: &ConnectionConfig) -> Result<ExitStatus> {
    let mut command = match &connection.auth {
        AuthConfig::InteractivePassword => base_ssh_command(connection),
        AuthConfig::KeyFile { path } => {
            let mut command = base_ssh_command(connection);
            command.arg("-i").arg(path);
            command
        }
        AuthConfig::StoredPassword { password } => {
            let mut command = Command::new("sshpass");
            command
                .arg("-p")
                .arg(password)
                .arg("ssh")
                .arg("-p")
                .arg(connection.port.to_string())
                .arg(format!("{}@{}", connection.user, connection.host));
            command
        }
    };

    command.status().context("failed to launch ssh")
}

fn base_ssh_command(connection: &ConnectionConfig) -> Command {
    let mut command = Command::new("ssh");
    command
        .arg("-p")
        .arg(connection.port.to_string())
        .arg(format!("{}@{}", connection.user, connection.host));
    command
}
