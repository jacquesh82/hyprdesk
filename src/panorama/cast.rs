//! Cast vers la zone vidéo du Panorama.
//!
//! Un vrai récepteur Google Cast est hors d'atteinte pour HBO Max, Disney+…
//! (certificat d'appareil signé par Google, DRM). hyprdesk fait comme un
//! Fire TV : le téléphone désigne le titre, la zone le lit avec le compte de
//! l'utilisateur. Un petit serveur du réseau local sert :
//!
//! - **Partager → hyprdesk** (MLG-217) : une page pour le téléphone, ouverte
//!   par le QR code de la zone (jeton secret dans l'adresse) ; on y colle le
//!   lien partagé par l'application, il s'ouvre dans la zone ; lecture/pause,
//!   ±10 s, fermer. Cible de partage du système (manifeste `share_target`)
//!   quand le navigateur du téléphone l'accepte.
//! - **Récepteur YouTube** (MLG-218) : un appareil DIAL (annonce SSDP,
//!   description, `/apps/YouTube`) ; l'icône cast de l'application YouTube
//!   le montre, et un lancement ouvre YouTube pour téléviseur
//!   (`youtube.com/tv`, appairé par le `pairingCode` reçu) dans la zone.
//!
//! Le serveur ne répond qu'aux adresses privées (réseau local) ; tout sauf
//! DIAL exige le jeton.

use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::sync::Mutex;
use std::time::Duration;

use iced::futures::{SinkExt, Stream};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream, UdpSocket};

use crate::control::Command;

/// Port HTTP du serveur (page du téléphone et DIAL).
pub const PORT: u16 = 8547;
/// YouTube pour téléviseur (application « leanback »).
pub const YOUTUBE_TV: &str = "https://www.youtube.com/tv";
/// Agent utilisateur de téléviseur : sans lui, `youtube.com/tv` renvoie
/// vers le site ordinaire.
pub const TV_AGENT: &str = "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) \
     Chrome/140.0.0.0 Safari/537.36 Cobalt/25.lts.30.1034943-gold (unlike Gecko) v8/8.8.278.17-jit \
     gles Starboard/15, hyprdesk_Panorama_2025/1.0 (hyprdesk, Panorama, Wired)";

const SSDP_GROUP: Ipv4Addr = Ipv4Addr::new(239, 255, 255, 250);
const DIAL_ST: &str = "urn:dial-multiscreen-org:service:dial:1";
/// Taille maximale d'une requête (en-têtes + corps).
const MAX_REQUEST: usize = 32 * 1024;

/// Dernier problème du serveur (port pris…), montré dans la zone.
static PROBLEM: Mutex<Option<String>> = Mutex::new(None);

/// Problème du serveur, s'il y en a un.
pub fn problem() -> Option<String> {
    PROBLEM.lock().ok().and_then(|p| p.clone())
}

fn set_problem(p: Option<String>) {
    if let Ok(mut g) = PROBLEM.lock() {
        *g = p;
    }
}

/// Cast actif (option `video.cast` du Panorama, active par défaut).
pub fn enabled(cfg: &crate::config::Config) -> bool {
    super::modules::option(cfg, "video", "cast") != "off"
}

/// L'adresse est YouTube pour téléviseur.
pub fn is_youtube_tv(url: &str) -> bool {
    let rest = url.split_once("://").map(|(_, r)| r).unwrap_or(url);
    let path = rest.find('/').map(|i| &rest[i..]).unwrap_or("");
    super::video::host(url) == "www.youtube.com"
        && (path == "/tv"
            || path.starts_with("/tv?")
            || path.starts_with("/tv/")
            || path.starts_with("/tv#"))
}

/// Options du navigateur de YouTube pour téléviseur : profil à part, agent
/// de téléviseur, lecture sans geste (le téléphone lance la vidéo), aucune
/// mise à jour de composant (relance de Brave).
pub fn tv_browser_args() -> Vec<String> {
    let dir = std::env::var_os("XDG_DATA_HOME")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| {
            std::path::PathBuf::from(std::env::var_os("HOME").unwrap_or_default())
                .join(".local/share")
        })
        .join("hyprdesk/youtube-tv");
    let mut args = vec![
        format!("--user-data-dir={}", dir.display()),
        format!("--user-agent={TV_AGENT}"),
        "--autoplay-policy=no-user-gesture-required".into(),
        "--no-first-run".into(),
        "--no-default-browser-check".into(),
        "--password-store=basic".into(),
        // Sans composants téléchargés : à l'arrivée de Widevine, Brave se
        // relançait seul et perdait la page (« New Tab » hors de la zone).
        "--disable-component-update".into(),
    ];
    // Wayland imposé : le service de hyprdesk n'a pas XDG_SESSION_TYPE, et
    // sans lui Brave s'ouvrait en X11 (classe « Brave-browser ») ; la
    // fenêtre, méconnue, restait hors de la zone.
    if std::env::var_os("WAYLAND_DISPLAY").is_some() {
        args.push("--ozone-platform=wayland".into());
    }
    args
}

/// Jeton secret de la page du téléphone (`$XDG_STATE_HOME/hyprdesk/cast-token`),
/// créé au premier besoin.
pub fn token() -> String {
    let path = super::super::layout::state_dir().join("cast-token");
    if let Ok(t) = std::fs::read_to_string(&path) {
        let t = t.trim();
        if t.len() >= 16 && t.chars().all(|c| c.is_ascii_alphanumeric()) {
            return t.to_string();
        }
    }
    let t: String = (0..24)
        .map(|_| char::from_digit(rand::random_range(0..36u32), 36).unwrap_or('0'))
        .collect();
    if let Some(dir) = path.parent() {
        let _ = std::fs::create_dir_all(dir);
    }
    {
        use std::io::Write;
        use std::os::unix::fs::OpenOptionsExt;
        if let Ok(mut f) = std::fs::OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .mode(0o600)
            .open(&path)
        {
            let _ = f.write_all(t.as_bytes());
        }
    }
    t
}

/// Adresse locale utilisée pour joindre `to` (aucun paquet envoyé).
fn local_toward(to: IpAddr) -> Option<IpAddr> {
    let s = std::net::UdpSocket::bind((Ipv4Addr::UNSPECIFIED, 0)).ok()?;
    s.connect((to, 9)).ok()?;
    s.local_addr().ok().map(|a| a.ip())
}

/// Adresses IPv4 privées des interfaces actives (hors boucle locale).
fn private_ipv4s() -> Vec<Ipv4Addr> {
    let mut out = Vec::new();
    // SAFETY : liste rendue par getifaddrs, parcourue puis libérée une fois.
    unsafe {
        let mut list: *mut libc::ifaddrs = std::ptr::null_mut();
        if libc::getifaddrs(&mut list) != 0 {
            return out;
        }
        let mut cur = list;
        while !cur.is_null() {
            let ifa = &*cur;
            let up = ifa.ifa_flags & libc::IFF_UP as u32 != 0;
            if up && !ifa.ifa_addr.is_null() && (*ifa.ifa_addr).sa_family as i32 == libc::AF_INET {
                let sin = &*(ifa.ifa_addr as *const libc::sockaddr_in);
                let ip = Ipv4Addr::from(u32::from_be(sin.sin_addr.s_addr));
                if ip.is_private() && !out.contains(&ip) {
                    out.push(ip);
                }
            }
            cur = ifa.ifa_next;
        }
        libc::freeifaddrs(list);
    }
    out
}

/// Adresse du bureau sur le réseau local : celle de la route par défaut,
/// sinon la première adresse privée (192.168… d'abord).
pub fn lan_ip() -> Option<IpAddr> {
    if let Some(ip @ IpAddr::V4(v4)) = local_toward(IpAddr::V4(Ipv4Addr::new(1, 1, 1, 1)))
        && v4.is_private()
    {
        return Some(ip);
    }
    let mut all = private_ipv4s();
    all.sort_by_key(|ip| (ip.octets()[0] != 192, ip.octets()));
    all.first().map(|ip| IpAddr::V4(*ip))
}

/// Adresse de la page du téléphone (celle du QR code).
pub fn page_url() -> Option<String> {
    Some(format!("http://{}:{PORT}/?k={}", lan_ip()?, token()))
}

/// Nom de l'appareil dans la liste de cast : « hyprdesk (machine) ».
pub fn device_name() -> String {
    let host = std::fs::read_to_string("/etc/hostname")
        .ok()
        .map(|h| h.trim().to_string())
        .filter(|h| !h.is_empty());
    match host {
        Some(h) => format!("hyprdesk ({h})"),
        None => "hyprdesk".into(),
    }
}

/// Identifiant UPnP stable, tiré de /etc/machine-id.
fn device_uuid() -> String {
    let id: String = std::fs::read_to_string("/etc/machine-id")
        .unwrap_or_default()
        .chars()
        .filter(|c| c.is_ascii_hexdigit())
        .chain(std::iter::repeat('0'))
        .take(32)
        .collect();
    format!(
        "{}-{}-{}-{}-{}",
        &id[0..8],
        &id[8..12],
        &id[12..16],
        &id[16..20],
        &id[20..32]
    )
}

/// Le pair est sur le réseau local (ou la machine elle-même).
fn private(ip: IpAddr) -> bool {
    match ip {
        IpAddr::V4(v4) => v4.is_private() || v4.is_loopback() || v4.is_link_local(),
        IpAddr::V6(v6) => {
            if let Some(v4) = v6.to_ipv4_mapped() {
                return private(IpAddr::V4(v4));
            }
            let s = v6.segments()[0];
            v6.is_loopback() || (s & 0xfe00) == 0xfc00 || (s & 0xffc0) == 0xfe80
        }
    }
}

/// Premier lien http(s) d'un texte partagé (« Regardez X sur Disney+ :
/// https://… »), ponctuation finale ôtée.
pub fn first_url(text: &str) -> Option<String> {
    let start = text
        .match_indices("http")
        .map(|(i, _)| i)
        .find(|&i| text[i..].starts_with("https://") || text[i..].starts_with("http://"))?;
    let url: String = text[start..]
        .chars()
        .take_while(|c| !c.is_whitespace() && !c.is_control() && !"\"'<>`«»“”".contains(*c))
        .collect();
    let url = url.trim_end_matches(['.', ',', ';', ':', ')', ']', '!', '?']);
    let host = super::video::host(url);
    (!host.is_empty() && url.len() <= 4096).then(|| url.to_string())
}

/// Décodage `application/x-www-form-urlencoded`.
fn url_decode(s: &str) -> String {
    let b = s.as_bytes();
    let mut out = Vec::with_capacity(b.len());
    let mut i = 0;
    while i < b.len() {
        match b[i] {
            b'+' => out.push(b' '),
            b'%' if i + 2 < b.len() => {
                let hex = |c: u8| (c as char).to_digit(16);
                match (hex(b[i + 1]), hex(b[i + 2])) {
                    (Some(h), Some(l)) => {
                        out.push((h * 16 + l) as u8);
                        i += 2;
                    }
                    _ => out.push(b'%'),
                }
            }
            c => out.push(c),
        }
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

fn xml_escape(s: &str) -> String {
    html_escape(s).replace('\'', "&apos;")
}

/// Paramètres de lancement DIAL de YouTube, gardés tels quels s'ils ne
/// contiennent que des caractères d'une requête encodée.
fn launch_params(body: &str) -> Option<String> {
    let body = body.trim();
    (body.len() <= 4096
        && body
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || "-._~%&=+".contains(c)))
    .then(|| body.to_string())
}

/// Requête HTTP lue.
#[derive(Debug, Default)]
struct Request {
    method: String,
    path: String,
    query: Vec<(String, String)>,
    headers: Vec<(String, String)>,
    body: String,
}

impl Request {
    fn query(&self, k: &str) -> Option<&str> {
        self.query
            .iter()
            .find(|(n, _)| n == k)
            .map(|(_, v)| v.as_str())
    }

    fn header(&self, k: &str) -> Option<&str> {
        self.headers
            .iter()
            .find(|(n, _)| n.eq_ignore_ascii_case(k))
            .map(|(_, v)| v.as_str())
    }
}

fn parse_head(head: &str) -> Option<Request> {
    let mut lines = head.split("\r\n");
    let mut first = lines.next()?.split(' ');
    let method = first.next()?.to_string();
    let target = first.next()?;
    let (path, q) = target.split_once('?').unwrap_or((target, ""));
    let query = q
        .split('&')
        .filter(|p| !p.is_empty())
        .map(|p| {
            let (k, v) = p.split_once('=').unwrap_or((p, ""));
            (url_decode(k), url_decode(v))
        })
        .collect();
    let headers = lines
        .filter_map(|l| l.split_once(':'))
        .map(|(k, v)| (k.trim().to_string(), v.trim().to_string()))
        .collect();
    Some(Request {
        method,
        path: path.to_string(),
        query,
        headers,
        body: String::new(),
    })
}

async fn read_request(s: &mut TcpStream) -> Option<Request> {
    let mut buf = Vec::with_capacity(2048);
    let mut chunk = [0u8; 4096];
    let end = loop {
        let n = s.read(&mut chunk).await.ok()?;
        if n == 0 {
            return None;
        }
        buf.extend_from_slice(&chunk[..n]);
        if let Some(i) = buf.windows(4).position(|w| w == b"\r\n\r\n") {
            break i;
        }
        if buf.len() > MAX_REQUEST {
            return None;
        }
    };
    let mut req = parse_head(&String::from_utf8_lossy(&buf[..end]))?;
    let len: usize = req
        .header("content-length")
        .and_then(|v| v.parse().ok())
        .unwrap_or(0);
    if len > MAX_REQUEST {
        return None;
    }
    let mut body = buf[end + 4..].to_vec();
    while body.len() < len {
        let n = s.read(&mut chunk).await.ok()?;
        if n == 0 {
            break;
        }
        body.extend_from_slice(&chunk[..n]);
    }
    body.truncate(len);
    req.body = String::from_utf8_lossy(&body).into_owned();
    Some(req)
}

struct Reply {
    status: &'static str,
    ctype: &'static str,
    headers: Vec<(&'static str, String)>,
    body: String,
}

impl Reply {
    fn new(status: &'static str, ctype: &'static str, body: impl Into<String>) -> Self {
        Self {
            status,
            ctype,
            headers: Vec::new(),
            body: body.into(),
        }
    }

    fn with(mut self, k: &'static str, v: String) -> Self {
        self.headers.push((k, v));
        self
    }

    fn empty(status: &'static str) -> Self {
        Self::new(status, "text/plain; charset=utf-8", "")
    }

    async fn send(self, s: &mut TcpStream) {
        let mut head = format!(
            "HTTP/1.1 {}\r\nContent-Type: {}\r\nContent-Length: {}\r\nCache-Control: no-store\r\nConnection: close\r\n",
            self.status,
            self.ctype,
            self.body.len()
        );
        for (k, v) in &self.headers {
            head.push_str(&format!("{k}: {v}\r\n"));
        }
        head.push_str("\r\n");
        let _ = s.write_all(head.as_bytes()).await;
        let _ = s.write_all(self.body.as_bytes()).await;
        let _ = s.shutdown().await;
    }
}

/// Une fenêtre YouTube pour téléviseur est ouverte.
async fn youtube_running() -> bool {
    tokio::task::spawn_blocking(|| {
        hyprpanel_core::ipc::query_json("clients")
            .ok()
            .and_then(|v| {
                v.as_array().map(|l| {
                    l.iter().any(|c| {
                        let class = c["class"].as_str().unwrap_or("").to_lowercase();
                        class.contains("www.youtube.com__tv")
                    })
                })
            })
            .unwrap_or(false)
    })
    .await
    .unwrap_or(false)
}

/// Ce qui joue (page du téléphone).
async fn now_playing() -> String {
    match crate::widgets::media::status().await {
        Some(s) => serde_json::json!({
            "title": s.title,
            "artist": s.artist,
            "playing": s.playing,
        })
        .to_string(),
        None => "{}".into(),
    }
}

async fn control(action: &str) -> bool {
    use crate::widgets::media;
    let Some(s) = media::status().await else {
        return false;
    };
    match action {
        "playpause" => media::media_control(s.bus, "PlayPause").await,
        "back" => {
            let _ = media::seek(&s.bus, -10).await;
        }
        "forward" => {
            let _ = media::seek(&s.bus, 10).await;
        }
        _ => return false,
    }
    true
}

type Commands = tokio::sync::mpsc::UnboundedSender<Command>;

/// Ouvre `url` dans la zone vidéo.
fn open(tx: &Commands, url: &str, name: &str) {
    let line = format!("open {url} {name}");
    let _ = tx.send(Command::Panorama(line.trim().to_string()));
}

async fn handle(
    mut s: TcpStream,
    peer: SocketAddr,
    key: &str,
    name: &str,
    uuid: &str,
    tx: &Commands,
) {
    if !private(peer.ip()) {
        return;
    }
    let local = s.local_addr().map(|a| a.ip()).ok();
    let Ok(Some(req)) = tokio::time::timeout(Duration::from_secs(5), read_request(&mut s)).await
    else {
        return;
    };
    let base = match local {
        Some(IpAddr::V6(v6)) => match v6.to_ipv4_mapped() {
            Some(v4) => format!("http://{v4}:{PORT}"),
            None => format!("http://[{v6}]:{PORT}"),
        },
        Some(ip) => format!("http://{ip}:{PORT}"),
        None => format!("http://127.0.0.1:{PORT}"),
    };
    let share_path = format!("/share/{key}");
    let authed = req.query("k") == Some(key) || req.header("x-hyprdesk-key") == Some(key);
    let reply = match (req.method.as_str(), req.path.as_str()) {
        // DIAL : description de l'appareil, application YouTube.
        ("GET", "/dd.xml") => Reply::new(
            "200 OK",
            "application/xml; charset=utf-8",
            device_xml(name, uuid),
        )
        .with("Application-URL", format!("{base}/apps/"))
        .with("Access-Control-Expose-Headers", "Application-URL".into()),
        ("GET", "/apps/YouTube") => {
            let state = if youtube_running().await {
                "running"
            } else {
                "stopped"
            };
            Reply::new("200 OK", "text/xml; charset=utf-8", app_xml(state))
        }
        ("POST", "/apps/YouTube") => match launch_params(&req.body) {
            Some(p) => {
                let url = if p.is_empty() {
                    YOUTUBE_TV.to_string()
                } else {
                    format!("{YOUTUBE_TV}?{p}")
                };
                tracing::info!("cast : YouTube lancé depuis {}", peer.ip());
                open(tx, &url, "YouTube");
                Reply::empty("201 Created").with("Location", format!("{base}/apps/YouTube/run"))
            }
            None => Reply::empty("413 Payload Too Large"),
        },
        ("DELETE", "/apps/YouTube/run") => {
            if youtube_running().await {
                let _ = tx.send(Command::Panorama("video close".into()));
            }
            Reply::empty("200 OK")
        }
        (_, p) if p.starts_with("/apps/") => Reply::empty("404 Not Found"),
        // Page du téléphone.
        ("GET", "/") if authed => Reply::new("200 OK", "text/html; charset=utf-8", page(key, name)),
        ("GET", "/") => Reply::new("403 Forbidden", "text/html; charset=utf-8", locked_page()),
        ("GET", "/manifest.webmanifest") if authed => {
            Reply::new("200 OK", "application/manifest+json", manifest(key, name))
        }
        ("GET", "/sw.js") => Reply::new(
            "200 OK",
            "text/javascript",
            "self.addEventListener('fetch', () => {});",
        ),
        ("GET", "/icon.svg") => Reply::new("200 OK", "image/svg+xml", ICON),
        // Cible de partage (manifeste) : le jeton est dans le chemin.
        ("GET", p) if p == share_path => {
            let shared = ["url", "text", "title"]
                .iter()
                .filter_map(|k| req.query(k))
                .find_map(first_url);
            let ok = shared.is_some();
            if let Some(u) = shared {
                open(tx, &u, "");
            }
            Reply::new("200 OK", "text/html; charset=utf-8", shared_page(key, ok))
        }
        ("POST", "/api/open") if authed => match first_url(&req.body) {
            Some(u) => {
                tracing::info!("cast : {u} envoyé depuis {}", peer.ip());
                open(tx, &u, "");
                Reply::empty("200 OK")
            }
            None => Reply::empty("400 Bad Request"),
        },
        ("POST", "/api/close") if authed => {
            let _ = tx.send(Command::Panorama("video close".into()));
            Reply::empty("200 OK")
        }
        ("POST", "/api/ctl") if authed => {
            if control(req.query("a").unwrap_or("")).await {
                Reply::empty("200 OK")
            } else {
                Reply::empty("409 Conflict")
            }
        }
        ("GET", "/api/now") if authed => {
            Reply::new("200 OK", "application/json", now_playing().await)
        }
        (_, "/api/open" | "/api/close" | "/api/ctl" | "/api/now" | "/manifest.webmanifest") => {
            Reply::empty("403 Forbidden")
        }
        _ => Reply::empty("404 Not Found"),
    };
    reply.send(&mut s).await;
}

fn device_xml(name: &str, uuid: &str) -> String {
    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<root xmlns="urn:schemas-upnp-org:device-1-0">
  <specVersion><major>1</major><minor>0</minor></specVersion>
  <device>
    <deviceType>urn:dial-multiscreen-org:device:dial:1</deviceType>
    <friendlyName>{}</friendlyName>
    <manufacturer>mindlog.today</manufacturer>
    <modelName>hyprdesk Panorama</modelName>
    <UDN>uuid:{uuid}</UDN>
    <serviceList>
      <service>
        <serviceType>urn:dial-multiscreen-org:service:dial:1</serviceType>
        <serviceId>urn:dial-multiscreen-org:serviceId:dial</serviceId>
        <controlURL>/ssdp/notfound</controlURL>
        <eventSubURL>/ssdp/notfound</eventSubURL>
        <SCPDURL>/ssdp/notfound</SCPDURL>
      </service>
    </serviceList>
  </device>
</root>
"#,
        xml_escape(name)
    )
}

fn app_xml(state: &str) -> String {
    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<service xmlns="urn:dial-multiscreen-org:schemas:dial" dialVer="2.1">
  <name>YouTube</name>
  <options allowStop="true"/>
  <state>{state}</state>{}
</service>
"#,
        if state == "running" {
            "\n  <link rel=\"run\" href=\"run\"/>"
        } else {
            ""
        }
    )
}

const ICON: &str = r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 96 96"><rect width="96" height="96" rx="22" fill="#1e2230"/><path d="M18 30a6 6 0 0 1 6-6h48a6 6 0 0 1 6 6v36a6 6 0 0 1-6 6H52v-6h20V30H24v6h-6z" fill="#8ab4f8"/><path d="M18 46a26 26 0 0 1 26 26h-6a20 20 0 0 0-20-20zm0 12a14 14 0 0 1 14 14h-6a8 8 0 0 0-8-8zm0 10a4 4 0 0 1 4 4h-4z" fill="#8ab4f8"/></svg>"##;

fn manifest(key: &str, name: &str) -> String {
    serde_json::json!({
        "name": name,
        "short_name": "hyprdesk",
        "start_url": format!("/?k={key}"),
        "scope": "/",
        "display": "standalone",
        "background_color": "#12141c",
        "theme_color": "#12141c",
        "icons": [{ "src": "/icon.svg", "sizes": "any", "type": "image/svg+xml" }],
        "share_target": {
            "action": format!("/share/{key}"),
            "method": "GET",
            "params": { "title": "title", "text": "text", "url": "url" }
        }
    })
    .to_string()
}

const STYLE: &str = "*{box-sizing:border-box}body{margin:0;font:16px system-ui,sans-serif;background:#12141c;color:#e6e8ef;padding:20px;max-width:560px;margin:auto}\
h1{font-size:20px;margin:4px 0 16px}p{color:#a3a8b8;line-height:1.4}textarea{width:100%;min-height:84px;border-radius:12px;border:1px solid #333a4d;background:#1b1f2b;color:inherit;padding:12px;font:inherit}\
button{font:inherit;border:0;border-radius:12px;padding:14px 16px;background:#2a3042;color:inherit}button.main{background:#8ab4f8;color:#10131a;font-weight:600;width:100%;margin-top:10px}\
.row{display:flex;gap:10px;margin-top:10px}.row button{flex:1}#now{margin:18px 0 6px;font-weight:600}#msg{min-height:1.4em;margin-top:10px}.card{background:#1b1f2b;border-radius:14px;padding:14px;margin-top:18px}";

fn page(key: &str, name: &str) -> String {
    let tr = |s: &str| html_escape(crate::i18n::tr(s));
    let k = serde_json::to_string(key).unwrap_or_else(|_| "\"\"".into());
    let msgs = serde_json::json!({
        "sent": crate::i18n::tr("Envoyé à la zone vidéo."),
        "nolink": crate::i18n::tr("Aucun lien http(s) dans ce texte."),
        "fail": crate::i18n::tr("hyprdesk ne répond pas."),
        "nothing": crate::i18n::tr("Rien ne joue."),
    });
    format!(
        r##"<!doctype html><html lang="{lang}"><head><meta charset="utf-8">
<meta name="viewport" content="width=device-width,initial-scale=1">
<meta name="theme-color" content="#12141c">
<link rel="manifest" href="/manifest.webmanifest?k={key}">
<link rel="icon" href="/icon.svg">
<title>{title}</title><style>{STYLE}</style></head><body>
<h1>{title}</h1>
<p>{how}</p>
<textarea id="u" placeholder="https://…"></textarea>
<button class="main" id="send">{send}</button>
<button id="paste" style="width:100%;margin-top:10px;display:none">{paste}</button>
<div id="msg"></div>
<div class="card">
<div id="now"></div>
<div class="row"><button data-a="back">−10 s</button><button data-a="playpause">⏯</button><button data-a="forward">+10 s</button></div>
<div class="row"><button id="close">{close}</button></div>
</div>
<p class="card">{yt}</p>
<script>
const K={k},M={msgs};
const $=id=>document.getElementById(id);
const say=t=>{{$('msg').textContent=t}};
async function post(p,body){{try{{const r=await fetch(p,{{method:'POST',headers:{{'X-Hyprdesk-Key':K}},body:body||''}});return r.status}}catch(e){{return 0}}}}
async function send(){{const v=$('u').value.trim();if(!v)return;const s=await post('/api/open',v);say(s==200?M.sent:s==400?M.nolink:M.fail);if(s==200)$('u').value=''}}
$('send').onclick=send;
if(navigator.clipboard&&navigator.clipboard.readText){{$('paste').style.display='block';$('paste').onclick=async()=>{{try{{$('u').value=await navigator.clipboard.readText();send()}}catch(e){{}}}}}}
document.querySelectorAll('[data-a]').forEach(b=>b.onclick=async()=>{{const s=await post('/api/ctl?a='+b.dataset.a);if(s!=200)say(s?M.nothing:M.fail);setTimeout(now,400)}});
$('close').onclick=()=>post('/api/close');
async function now(){{try{{const r=await fetch('/api/now',{{headers:{{'X-Hyprdesk-Key':K}}}});const j=await r.json();$('now').textContent=j.title?((j.playing?'▶ ':'⏸ ')+j.title+(j.artist?' — '+j.artist:'')):M.nothing}}catch(e){{}}}}
now();setInterval(now,4000);
if('serviceWorker' in navigator)navigator.serviceWorker.register('/sw.js').catch(()=>{{}});
</script></body></html>"##,
        lang = if crate::i18n::tr("Envoyer") == "Envoyer" {
            "fr"
        } else {
            "en"
        },
        key = html_escape(key),
        title = tr("Zone vidéo de hyprdesk"),
        how = tr(
            "Dans Disney+, HBO Max, Netflix… : Partager → Copier le lien, puis collez-le ici. Le titre s'ouvre dans la zone vidéo du Panorama, avec votre compte."
        ),
        send = tr("Envoyer"),
        paste = tr("Coller et envoyer"),
        close = tr("Fermer la vidéo"),
        yt = html_escape(&crate::tr!(
            "YouTube : touchez l'icône cast de l'application et choisissez « {name} ».",
            name = name.to_string()
        )),
    )
}

fn locked_page() -> String {
    format!(
        r#"<!doctype html><html><head><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1"><style>{STYLE}</style></head><body><h1>hyprdesk</h1><p>{}</p></body></html>"#,
        html_escape(crate::i18n::tr(
            "Scannez le QR code affiché dans la zone vidéo du Panorama."
        ))
    )
}

fn shared_page(key: &str, ok: bool) -> String {
    let msg = if ok {
        crate::i18n::tr("Envoyé à la zone vidéo.")
    } else {
        crate::i18n::tr("Aucun lien http(s) dans ce texte.")
    };
    format!(
        r#"<!doctype html><html><head><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1"><style>{STYLE}</style></head><body><h1>hyprdesk</h1><p>{}</p><p><a style="color:#8ab4f8" href="/?k={}">{}</a></p></body></html>"#,
        html_escape(msg),
        html_escape(key),
        html_escape(crate::i18n::tr("Télécommande")),
    )
}

/// Socket SSDP (UDP 1900, groupe 239.255.255.250 sur chaque interface
/// privée), partagée avec les autres programmes qui l'écoutent.
fn ssdp_socket() -> std::io::Result<UdpSocket> {
    use socket2::{Domain, Protocol, Socket, Type};
    let s = Socket::new(Domain::IPV4, Type::DGRAM, Some(Protocol::UDP))?;
    s.set_reuse_address(true)?;
    s.set_reuse_port(true)?;
    s.set_nonblocking(true)?;
    s.bind(&SocketAddr::from((Ipv4Addr::UNSPECIFIED, 1900)).into())?;
    let mut joined = 0;
    for ip in private_ipv4s() {
        if s.join_multicast_v4(&SSDP_GROUP, &ip).is_ok() {
            joined += 1;
        }
    }
    if joined == 0 {
        s.join_multicast_v4(&SSDP_GROUP, &Ipv4Addr::UNSPECIFIED)?;
    }
    UdpSocket::from_std(s.into())
}

fn header_value<'a>(msg: &'a str, name: &str) -> Option<&'a str> {
    msg.lines()
        .filter_map(|l| l.split_once(':'))
        .find(|(k, _)| k.trim().eq_ignore_ascii_case(name))
        .map(|(_, v)| v.trim())
}

/// Réponse à une recherche SSDP, si elle concerne l'appareil.
fn ssdp_answer(msg: &str, uuid: &str, location: &str) -> Option<String> {
    if !msg.starts_with("M-SEARCH") {
        return None;
    }
    let st = header_value(msg, "ST")?;
    let udn = format!("uuid:{uuid}");
    let st = match st {
        DIAL_ST | "ssdp:all" => DIAL_ST.to_string(),
        "upnp:rootdevice" | "urn:dial-multiscreen-org:device:dial:1" => st.to_string(),
        s if s == udn => udn.clone(),
        _ => return None,
    };
    let usn = if st == udn {
        udn.clone()
    } else {
        format!("{udn}::{st}")
    };
    Some(format!(
        "HTTP/1.1 200 OK\r\nCACHE-CONTROL: max-age=1800\r\nEXT:\r\nLOCATION: {location}\r\n\
         SERVER: Linux UPnP/1.1 hyprdesk/{}\r\nST: {st}\r\nUSN: {usn}\r\n\
         BOOTID.UPNP.ORG: 1\r\nCONFIGID.UPNP.ORG: 1\r\n\r\n",
        env!("CARGO_PKG_VERSION")
    ))
}

async fn ssdp(uuid: String) {
    let sock = match ssdp_socket() {
        Ok(s) => s,
        Err(e) => {
            tracing::warn!("cast : SSDP (UDP 1900) : {e}");
            set_problem(Some(crate::tr!(
                "Récepteur YouTube indisponible (UDP 1900) : {e}",
                e = e
            )));
            return;
        }
    };
    let mut buf = [0u8; 2048];
    loop {
        let Ok((n, from)) = sock.recv_from(&mut buf).await else {
            tokio::time::sleep(Duration::from_millis(200)).await;
            continue;
        };
        if !private(from.ip()) {
            continue;
        }
        let msg = String::from_utf8_lossy(&buf[..n]);
        let Some(local) = local_toward(from.ip()) else {
            continue;
        };
        let location = format!("http://{local}:{PORT}/dd.xml");
        if let Some(answer) = ssdp_answer(&msg, &uuid, &location) {
            let _ = sock.send_to(answer.as_bytes(), from).await;
        }
    }
}

/// Tâches du serveur, arrêtées avec l'abonnement : sinon elles gardaient
/// le port et envoyaient leurs commandes dans le vide.
struct Tasks(Vec<tokio::task::JoinHandle<()>>);

impl Drop for Tasks {
    fn drop(&mut self) {
        for t in &self.0 {
            t.abort();
        }
    }
}

/// Écoute sur le port du cast ; le serveur d'un abonnement qui vient de
/// s'arrêter peut le tenir encore un instant.
async fn bind() -> std::io::Result<TcpListener> {
    let mut tries = 0;
    loop {
        match TcpListener::bind((Ipv4Addr::UNSPECIFIED, PORT)).await {
            Err(e) if e.kind() == std::io::ErrorKind::AddrInUse && tries < 20 => {
                tries += 1;
                tokio::time::sleep(Duration::from_millis(250)).await;
            }
            r => return r,
        }
    }
}

/// Serveur du cast, tant que l'abonnement vit : commandes du Panorama
/// (`open <adresse>`, `video close`).
pub fn stream() -> impl Stream<Item = Command> {
    iced::stream::channel(
        16,
        async move |mut out: iced::futures::channel::mpsc::Sender<Command>| {
            let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel::<Command>();
            let key = tokio::task::spawn_blocking(token).await.unwrap_or_default();
            let name = device_name();
            let uuid = device_uuid();
            set_problem(None);
            let mut tasks = Tasks(Vec::new());
            match bind().await {
                Ok(listener) => {
                    tracing::info!("cast : page du téléphone et DIAL sur le port {PORT}");
                    tasks.0.push(tokio::spawn(ssdp(uuid.clone())));
                    let tx = tx.clone();
                    tasks.0.push(tokio::spawn(async move {
                        let slots = std::sync::Arc::new(tokio::sync::Semaphore::new(32));
                        loop {
                            let Ok((s, peer)) = listener.accept().await else {
                                tokio::time::sleep(Duration::from_millis(100)).await;
                                continue;
                            };
                            let Ok(permit) = slots.clone().try_acquire_owned() else {
                                continue;
                            };
                            let (key, name, uuid, tx) =
                                (key.clone(), name.clone(), uuid.clone(), tx.clone());
                            tokio::spawn(async move {
                                let _permit = permit;
                                handle(s, peer, &key, &name, &uuid, &tx).await;
                            });
                        }
                    }));
                }
                Err(e) => {
                    tracing::warn!("cast : port {PORT} : {e}");
                    set_problem(Some(crate::tr!(
                        "Cast indisponible : port {port} déjà pris ({e}).",
                        port = PORT,
                        e = e
                    )));
                }
            }
            drop(tx);
            while let Some(c) = rx.recv().await {
                let _ = out.send(c).await;
            }
            // Sans serveur (port pris) : l'abonnement reste, sans rien faire.
            let _tasks = tasks;
            std::future::pending::<()>().await;
        },
    )
}

/// QR code de `text` en RGBA (modules sombres sur fond clair, marge de 4),
/// `scale` pixels par module : (largeur, hauteur, pixels).
pub fn qr_rgba(text: &str, scale: u32) -> Option<(u32, u32, Vec<u8>)> {
    let code = qrcode::QrCode::with_error_correction_level(text, qrcode::EcLevel::M).ok()?;
    let n = code.width() as u32;
    let quiet = 4;
    let side = (n + quiet * 2) * scale;
    let colors = code.to_colors();
    let mut px = Vec::with_capacity((side * side * 4) as usize);
    for y in 0..side {
        for x in 0..side {
            let (mx, my) = (
                (x / scale) as i64 - quiet as i64,
                (y / scale) as i64 - quiet as i64,
            );
            let dark = mx >= 0
                && my >= 0
                && (mx as u32) < n
                && (my as u32) < n
                && colors[(my as u32 * n + mx as u32) as usize] == qrcode::Color::Dark;
            let v = if dark { 0x12 } else { 0xf4 };
            px.extend_from_slice(&[v, v, v, 0xff]);
        }
    }
    Some((side, side, px))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lien_partage() {
        assert_eq!(
            first_url("Regardez The Night Of sur Max : https://play.max.com/show/abc?x=1.")
                .as_deref(),
            Some("https://play.max.com/show/abc?x=1")
        );
        assert_eq!(
            first_url("« https://www.disneyplus.com/browse/entity-1 »").as_deref(),
            Some("https://www.disneyplus.com/browse/entity-1")
        );
        assert_eq!(first_url("pas de lien"), None);
        assert_eq!(first_url("http:// rien"), None);
    }

    #[test]
    fn youtube_tv() {
        assert!(is_youtube_tv("https://www.youtube.com/tv"));
        assert!(is_youtube_tv(
            "https://www.youtube.com/tv?pairingCode=abc&theme=cl"
        ));
        assert!(!is_youtube_tv("https://www.youtube.com/watch?v=x"));
        assert!(!is_youtube_tv("https://www.youtube.com/tvshows"));
        assert!(!is_youtube_tv("https://m.youtube.com/tv"));
    }

    #[test]
    fn parametres_dial() {
        assert_eq!(
            launch_params("pairingCode=1a-2b&v=dQw4w9WgXcQ&t=0&theme=cl\r\n").as_deref(),
            Some("pairingCode=1a-2b&v=dQw4w9WgXcQ&t=0&theme=cl")
        );
        assert_eq!(launch_params("a=<script>"), None);
        assert_eq!(launch_params("a b"), None);
    }

    #[test]
    fn decodage() {
        assert_eq!(url_decode("a%20b+c%C3%A9"), "a b cé");
        assert_eq!(url_decode("100%"), "100%");
        assert_eq!(url_decode("%zz"), "%zz");
    }

    #[test]
    fn requete() {
        let r = parse_head("GET /share/k1?url=https%3A%2F%2Fx.fr%2Fa&text= HTTP/1.1\r\nHost: h\r\nX-Hyprdesk-Key: k1").unwrap();
        assert_eq!(r.path, "/share/k1");
        assert_eq!(r.query("url"), Some("https://x.fr/a"));
        assert_eq!(r.header("x-hyprdesk-key"), Some("k1"));
    }

    #[test]
    fn reseau_local() {
        assert!(private("192.168.1.20".parse().unwrap()));
        assert!(private("127.0.0.1".parse().unwrap()));
        assert!(private("::ffff:10.0.0.2".parse().unwrap()));
        assert!(private("fe80::1".parse().unwrap()));
        assert!(!private("8.8.8.8".parse().unwrap()));
        assert!(!private("2001:db8::1".parse().unwrap()));
    }

    #[test]
    fn recherche_ssdp() {
        let m = "M-SEARCH * HTTP/1.1\r\nHOST: 239.255.255.250:1900\r\nMAN: \"ssdp:discover\"\r\nMX: 1\r\nST: urn:dial-multiscreen-org:service:dial:1\r\n\r\n";
        let a = ssdp_answer(m, "u-1", "http://192.168.1.2:8547/dd.xml").unwrap();
        assert!(a.contains("LOCATION: http://192.168.1.2:8547/dd.xml\r\n"));
        assert!(a.contains("USN: uuid:u-1::urn:dial-multiscreen-org:service:dial:1\r\n"));
        assert!(ssdp_answer(&m.replace("dial:1", "x"), "u-1", "l").is_none());
        assert!(ssdp_answer("NOTIFY * HTTP/1.1\r\nST: ssdp:all\r\n", "u-1", "l").is_none());
    }

    #[test]
    fn qr() {
        let (w, h, px) = qr_rgba("http://192.168.1.2:8547/?k=abc", 3).unwrap();
        assert_eq!(w, h);
        assert_eq!(px.len(), (w * h * 4) as usize);
    }
}
