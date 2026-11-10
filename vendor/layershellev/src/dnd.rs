//! Glisser-déposer de fichiers vers les surfaces (ajout de hyprdesk) :
//! `wl_data_device`, offres en `text/uri-list`. Entrée, mouvement, sortie et
//! dépôt deviennent des `DispatchMessage::Dnd*` ; au dépôt, la liste des
//! fichiers est lue auprès de l'application source (délai borné).

use std::collections::HashMap;
use std::io::Read;
use std::os::fd::AsFd;
use std::path::PathBuf;
use std::time::Duration;

use wayland_client::backend::ObjectId;
use wayland_client::protocol::wl_data_device::{self, WlDataDevice};
use wayland_client::protocol::wl_data_device_manager::{DndAction, WlDataDeviceManager};
use wayland_client::protocol::wl_data_offer::{self, WlDataOffer};
use wayland_client::{Connection, Dispatch, Proxy, QueueHandle, delegate_noop};

use crate::WindowState;
use crate::events::DispatchMessageInner;
use crate::id;

const URI_LIST: &str = "text/uri-list";

/// État du glisser-déposer en cours.
#[derive(Default, Debug)]
pub struct DndState {
    pub(crate) device: Option<WlDataDevice>,
    /// Types annoncés par chaque offre.
    offers: HashMap<ObjectId, Vec<String>>,
    current: Option<WlDataOffer>,
    surface: Option<id::Id>,
    pos: (f64, f64),
}

delegate_noop!(@<T> WindowState<T>: ignore WlDataDeviceManager);

impl<T> Dispatch<WlDataOffer, ()> for WindowState<T> {
    fn event(
        state: &mut Self,
        offer: &WlDataOffer,
        event: wl_data_offer::Event,
        _data: &(),
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
    ) {
        if let wl_data_offer::Event::Offer { mime_type } = event {
            state.dnd.offers.entry(offer.id()).or_default().push(mime_type);
        }
    }
}

impl<T: 'static> Dispatch<WlDataDevice, ()> for WindowState<T> {
    fn event(
        state: &mut Self,
        _device: &WlDataDevice,
        event: wl_data_device::Event,
        _data: &(),
        conn: &Connection,
        _qh: &QueueHandle<Self>,
    ) {
        match event {
            wl_data_device::Event::DataOffer { id } => {
                state.dnd.offers.insert(id.id(), Vec::new());
            }
            wl_data_device::Event::Enter {
                serial,
                surface,
                x,
                y,
                id,
            } => {
                let target = state.get_id_from_surface(&surface);
                state.dnd.surface = target;
                state.dnd.pos = (x, y);
                if let Some(offer) = &id {
                    let files = state
                        .dnd
                        .offers
                        .get(&offer.id())
                        .is_some_and(|m| m.iter().any(|t| t == URI_LIST));
                    if files {
                        if offer.version() >= 3 {
                            offer.set_actions(DndAction::Copy, DndAction::Copy);
                        }
                        offer.accept(serial, Some(URI_LIST.into()));
                    } else {
                        offer.accept(serial, None);
                    }
                }
                state.dnd.current = id;
                state
                    .message
                    .push((target, DispatchMessageInner::DndEnter { x, y }));
            }
            wl_data_device::Event::Motion { x, y, .. } => {
                state.dnd.pos = (x, y);
                state
                    .message
                    .push((state.dnd.surface, DispatchMessageInner::DndMotion { x, y }));
            }
            wl_data_device::Event::Leave => {
                if let Some(offer) = state.dnd.current.take() {
                    state.dnd.offers.remove(&offer.id());
                    offer.destroy();
                }
                state
                    .message
                    .push((state.dnd.surface.take(), DispatchMessageInner::DndLeave));
            }
            wl_data_device::Event::Drop => {
                let Some(offer) = state.dnd.current.take() else {
                    return;
                };
                let files = state
                    .dnd
                    .offers
                    .remove(&offer.id())
                    .is_some_and(|m| m.iter().any(|t| t == URI_LIST));
                let paths = if files { receive(&offer, conn) } else { Vec::new() };
                if offer.version() >= 3 && files {
                    offer.finish();
                }
                offer.destroy();
                let (x, y) = state.dnd.pos;
                state.message.push((
                    state.dnd.surface.take(),
                    DispatchMessageInner::DndDrop { x, y, paths },
                ));
            }
            wl_data_device::Event::Selection { id } => {
                // Presse-papiers : rien à en faire ici ; l'offre précédente est oubliée.
                if let Some(offer) = id {
                    state.dnd.offers.remove(&offer.id());
                }
            }
            _ => {}
        }
    }

    wayland_client::event_created_child!(WindowState<T>, WlDataDevice, [
        wl_data_device::EVT_DATA_OFFER_OPCODE => (WlDataOffer, ()),
    ]);
}

/// Lit la liste des fichiers auprès de l'application source (3 s au plus).
fn receive(offer: &WlDataOffer, conn: &Connection) -> Vec<PathBuf> {
    let Ok((mut read, write)) = std::os::unix::net::UnixStream::pair() else {
        return Vec::new();
    };
    offer.receive(URI_LIST.into(), write.as_fd());
    let _ = conn.flush();
    drop(write);
    let _ = read.set_read_timeout(Some(Duration::from_secs(3)));
    let mut buf = Vec::new();
    let _ = read.read_to_end(&mut buf);
    parse_uri_list(&String::from_utf8_lossy(&buf))
}

/// `text/uri-list` → chemins locaux (`file://`, encodage `%XX` décodé).
pub fn parse_uri_list(s: &str) -> Vec<PathBuf> {
    s.lines()
        .map(str::trim)
        .filter(|l| !l.is_empty() && !l.starts_with('#'))
        .filter_map(|l| l.strip_prefix("file://"))
        .map(|rest| {
            // `file://hote/chemin` : on garde le chemin.
            let path = if rest.starts_with('/') {
                rest
            } else {
                rest.find('/').map_or(rest, |i| &rest[i..])
            };
            PathBuf::from(percent_decode(path))
        })
        .collect()
}

fn percent_decode(s: &str) -> String {
    let bytes = s.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        let hex = |c: u8| (c as char).to_digit(16);
        if bytes[i] == b'%' && i + 2 < bytes.len() {
            if let (Some(h), Some(l)) = (hex(bytes[i + 1]), hex(bytes[i + 2])) {
                out.push((h * 16 + l) as u8);
                i += 3;
                continue;
            }
            out.push(bytes[i]);
            i += 1;
        } else {
            out.push(bytes[i]);
            i += 1;
        }
    }
    String::from_utf8_lossy(&out).into_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn liste_d_uri() {
        let l = parse_uri_list(
            "# commentaire\r\nfile:///home/moi/Mes%20Images/a.png\r\nfile://hote/tmp/b\nhttps://x\n",
        );
        assert_eq!(
            l,
            [
                PathBuf::from("/home/moi/Mes Images/a.png"),
                PathBuf::from("/tmp/b")
            ]
        );
        assert_eq!(percent_decode("%C3%A9t%C3%A9"), "été");
        assert_eq!(percent_decode("100%é"), "100%é");
        assert_eq!(percent_decode("fin%"), "fin%");
    }
}
