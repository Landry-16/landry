//! Auto-paste: Ctrl+V sent to the focused app through the RemoteDesktop portal.
//! A short session per paste (GNOME shows a "remote control" indicator while one is open);
//! the restore token saved after the first approval avoids asking again.

#![cfg_attr(windows, allow(dead_code))]

#[cfg(target_os = "linux")]
use ashpd::desktop::remote_desktop::{DeviceType, KeyState, RemoteDesktop, SelectDevicesOptions};
#[cfg(target_os = "linux")]
use ashpd::desktop::PersistMode;
use std::path::PathBuf;

const CONTROL_L: i32 = 0xffe3;
const KEY_V: i32 = 0x0076;
const LEFT: i32 = 0xff51;

fn token_path() -> PathBuf {
    crate::paths::data_dir().join("remote-desktop.token")
}

/// Not on Windows yet: the caller falls back to "paste with Ctrl+V".
#[cfg(windows)]
pub async fn send(_back: usize) -> Result<(), String> {
    Err("collage automatique indisponible sous Windows".into())
}

/// Press Ctrl+V, then Left `back` times (to land on the snippet's {curseur}).
#[cfg(target_os = "linux")]
pub async fn send(back: usize) -> ashpd::Result<()> {
    let rd = RemoteDesktop::new().await?;
    let session = rd.create_session(Default::default()).await?;
    let token = std::fs::read_to_string(token_path()).ok();
    rd.select_devices(
        &session,
        SelectDevicesOptions::default()
            .set_devices(Some(DeviceType::Keyboard.into()))
            .set_persist_mode(PersistMode::ExplicitlyRevoked)
            .set_restore_token(token.as_deref().map(str::trim)),
    )
    .await?
    .response()?;
    let started = rd.start(&session, None, Default::default()).await?.response()?;
    // Restore tokens are single-use: keep the new one.
    if let Some(t) = started.restore_token() {
        let _ = std::fs::create_dir_all(crate::paths::data_dir());
        let _ = std::fs::write(token_path(), t);
    }
    let key = |sym: i32, state: KeyState| rd.notify_keyboard_keysym(&session, sym, state, Default::default());
    key(CONTROL_L, KeyState::Pressed).await?;
    key(KEY_V, KeyState::Pressed).await?;
    key(KEY_V, KeyState::Released).await?;
    key(CONTROL_L, KeyState::Released).await?;
    for _ in 0..back {
        key(LEFT, KeyState::Pressed).await?;
        key(LEFT, KeyState::Released).await?;
    }
    session.close().await?;
    Ok(())
}
