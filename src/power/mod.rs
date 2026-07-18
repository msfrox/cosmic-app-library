// Copyright 2023 System76 <info@system76.com>
// SPDX-License-Identifier: GPL-3.0-only
//! Power/session actions, reused from cosmic-applet-power.

pub mod cosmic_session;
pub mod session_manager;

use logind_zbus::{
    manager::ManagerProxy,
    session::{SessionClass, SessionProxy, SessionType},
    user::UserProxy,
};
use nix::unistd::getuid;
use zbus::Connection;

use cosmic_session::CosmicSessionProxy;
use session_manager::SessionManagerProxy;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PowerAction {
    Lock,
    Suspend,
    LogOut,
    Restart,
    Shutdown,
}

impl PowerAction {
    /// The `cosmic-osd` subcommand for actions that show a confirmation
    /// dialog; `None` for actions that run immediately.
    pub fn osd_arg(self) -> Option<&'static str> {
        match self {
            PowerAction::LogOut => Some("log-out"),
            PowerAction::Restart => Some("restart"),
            PowerAction::Shutdown => Some("shutdown"),
            PowerAction::Lock | PowerAction::Suspend => None,
        }
    }

    pub async fn perform(self) -> zbus::Result<()> {
        match self {
            PowerAction::Lock => lock().await,
            PowerAction::Suspend => suspend().await,
            PowerAction::LogOut => log_out().await,
            PowerAction::Restart => restart().await,
            PowerAction::Shutdown => shutdown().await,
        }
    }
}

async fn restart() -> zbus::Result<()> {
    let connection = Connection::system().await?;
    let manager_proxy = ManagerProxy::new(&connection).await?;
    manager_proxy.reboot(true).await
}

async fn shutdown() -> zbus::Result<()> {
    let connection = Connection::system().await?;
    let manager_proxy = ManagerProxy::new(&connection).await?;
    manager_proxy.power_off(true).await
}

async fn suspend() -> zbus::Result<()> {
    let connection = Connection::system().await?;
    let manager_proxy = ManagerProxy::new(&connection).await?;
    manager_proxy.suspend(true).await
}

async fn lock() -> zbus::Result<()> {
    let connection = Connection::system().await?;
    let manager_proxy = ManagerProxy::new(&connection).await?;
    // Get the session this current process is running in
    let our_uid = getuid().as_raw() as u32;
    let user_path = manager_proxy.get_user(our_uid).await?;
    let user = UserProxy::builder(&connection)
        .path(user_path)?
        .build()
        .await?;
    // Lock all non-TTY sessions of this user
    let sessions = user.sessions().await?;
    let mut locked_successfully = false;
    for (_, session_path) in sessions {
        let Ok(session) = SessionProxy::builder(&connection)
            .path(session_path)?
            .build()
            .await
        else {
            continue;
        };

        if session.class().await == Ok(SessionClass::User)
            && session.type_().await? != SessionType::TTY
            && session.lock().await.is_ok()
        {
            locked_successfully = true;
        }
    }

    if locked_successfully {
        Ok(())
    } else {
        Err(zbus::Error::Failure("locking session failed".to_string()))
    }
}

async fn log_out() -> zbus::Result<()> {
    let session_type = std::env::var("XDG_CURRENT_DESKTOP").ok();
    let connection = Connection::session().await?;
    if let Some("pop:GNOME") = session_type.as_ref().map(|s| s.trim()) {
        let manager_proxy = SessionManagerProxy::new(&connection).await?;
        manager_proxy.logout(0).await?;
    } else {
        // By default assume COSMIC
        let cosmic_session = CosmicSessionProxy::new(&connection).await?;
        cosmic_session.exit().await?;
    }
    Ok(())
}
