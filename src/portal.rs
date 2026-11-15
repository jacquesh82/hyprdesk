//! Sélecteur de dossier graphique : `org.freedesktop.portal.FileChooser`
//! (xdg-desktop-portal, vérifié par `hyprdesk doctor`). Le portail ouvre la
//! boîte de dialogue de l'implémentation active (gtk, gnome, kde…).

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use iced::futures::StreamExt;
use zbus::zvariant::{OwnedValue, Value};

const DEST: &str = "org.freedesktop.portal.Desktop";
const PATH: &str = "/org/freedesktop/portal/desktop";

/// Demande un dossier à l'utilisateur. `None` : annulé, ou portail absent
/// (l'erreur est journalisée).
pub async fn pick_folder(title: String, start: Option<PathBuf>) -> Option<PathBuf> {
    match open(&title, start.as_deref(), true).await {
        Ok(p) => p,
        Err(e) => {
            tracing::warn!("sélecteur de dossier (portail) : {e}");
            None
        }
    }
}

async fn open(title: &str, start: Option<&Path>, directory: bool) -> zbus::Result<Option<PathBuf>> {
    let conn = zbus::Connection::session().await?;
    // Le chemin de la requête se déduit du nom unique et du jeton : on
    // s'abonne à la réponse AVANT l'appel, sans quoi elle peut filer.
    let token = format!("hyprdesk{}", rand::random::<u32>());
    let sender = conn
        .unique_name()
        .map(|n| n.trim_start_matches(':').replace('.', "_"))
        .unwrap_or_default();
    let request = format!("{PATH}/request/{sender}/{token}");
    let proxy: zbus::Proxy<'_> = zbus::proxy::Builder::new(&conn)
        .destination(DEST)?
        .path(request)?
        .interface("org.freedesktop.portal.Request")?
        .cache_properties(zbus::proxy::CacheProperties::No)
        .build()
        .await?;
    let mut responses = proxy.receive_signal("Response").await?;

    let mut options: HashMap<&str, Value<'_>> = HashMap::new();
    options.insert("handle_token", Value::from(token.as_str()));
    options.insert("directory", Value::from(directory));
    options.insert("modal", Value::from(true));
    if let Some(dir) = start.filter(|d| d.is_dir()) {
        // `ay` terminé par un octet nul.
        let mut bytes = dir.as_os_str().as_encoded_bytes().to_vec();
        bytes.push(0);
        options.insert("current_folder", Value::from(bytes));
    }
    conn.call_method(
        Some(DEST),
        PATH,
        Some("org.freedesktop.portal.FileChooser"),
        "OpenFile",
        &("", title, options),
    )
    .await?;

    let Some(msg) = responses.next().await else {
        return Ok(None);
    };
    let (code, results): (u32, HashMap<String, OwnedValue>) = msg.body().deserialize()?;
    if code != 0 {
        return Ok(None);
    }
    let uris: Vec<String> = match results.get("uris") {
        Some(v) => v.try_clone()?.try_into()?,
        None => return Ok(None),
    };
    Ok(uris.first().and_then(|u| file_uri_path(u)))
}

/// `file:///home/moi/Mes%20images` → `/home/moi/Mes images`.
fn file_uri_path(uri: &str) -> Option<PathBuf> {
    let rest = uri.strip_prefix("file://")?;
    // Hôte éventuel (`file://localhost/…`).
    let path = &rest[rest.find('/')?..];
    let bytes = path.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%'
            && let Some(hex) = bytes.get(i + 1..i + 3)
            && let Ok(hex) = std::str::from_utf8(hex)
            && let Ok(b) = u8::from_str_radix(hex, 16)
        {
            out.push(b);
            i += 3;
        } else {
            out.push(bytes[i]);
            i += 1;
        }
    }
    use std::os::unix::ffi::OsStringExt;
    Some(PathBuf::from(std::ffi::OsString::from_vec(out)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decodes_file_uris() {
        assert_eq!(
            file_uri_path("file:///home/moi/Mes%20images"),
            Some(PathBuf::from("/home/moi/Mes images"))
        );
        assert_eq!(
            file_uri_path("file://localhost/tmp/%C3%A9t%C3%A9"),
            Some(PathBuf::from("/tmp/été"))
        );
        assert_eq!(file_uri_path("https://x/y"), None);
    }
}
