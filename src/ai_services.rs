//! Autres services IA : les fournisseurs qui font les travaux ponctuels
//! confiés à un modèle, et la fonction de hyprdesk que chacun sert
//! (Réglages → IA → Autres services IA).
//!
//! - LYRA, toujours là : l'assistante du bureau, avec l'abonnement déjà
//!   choisi pour elle (`hyprpanel-chat cli`, un seul tour, sans accès aux
//!   fichiers). Toute fonction passe par elle tant qu'on n'en décide pas autrement.
//! - Autant de fournisseurs qu'on veut, Anthropic, OpenAI, Z.ai ou compatibles
//!   OpenAI (Ollama, LM Studio…) : nom, endpoint et modèle dans settings.lua
//!   (`ai_services`), clé API dans le trousseau (Secret Service,
//!   `secret-tool`), à défaut dans la variable d'environnement du type.
//! - `ai_use` : fonction → fournisseur (`smartui = "claude-perso"`).
//!
//! Les requêtes passent par `curl` ; la clé lui arrive par son entrée
//! standard, jamais dans la ligne de commande (visible de `ps`).

use std::io::Write as _;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use serde_json::{Value, json};

/// Délai d'une génération (un modèle qui réfléchit peut prendre une minute).
const TIMEOUT: Duration = Duration::from_secs(300);
/// Attribut du trousseau sous lequel hyprdesk range ses clés.
const KEYRING_SERVICE: &str = "hyprdesk-ai";
/// Version de l'API Anthropic (en-tête `anthropic-version`).
const ANTHROPIC_VERSION: &str = "2026-10-21";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Provider {
    /// L'abonnement de l'assistant, par LYRA.
    #[default]
    Lyra,
    Anthropic,
    OpenAi,
    Zai,
    /// N'importe quel service qui parle le protocole d'OpenAI (`/chat/completions`).
    Compatible,
}

#[cfg(test)]
pub const PROVIDERS: [Provider; 5] = [
    Provider::Lyra,
    Provider::Anthropic,
    Provider::OpenAi,
    Provider::Zai,
    Provider::Compatible,
];

impl Provider {
    pub fn parse(s: &str) -> Option<Self> {
        Some(match s.trim().to_lowercase().as_str() {
            "lyra" | "assistant" => Provider::Lyra,
            "anthropic" | "claude" => Provider::Anthropic,
            "openai" | "chatgpt" => Provider::OpenAi,
            "zai" | "z.ai" | "glm" => Provider::Zai,
            "compatible" | "custom" | "ollama" | "openai-compatible" => Provider::Compatible,
            _ => return None,
        })
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Provider::Lyra => "lyra",
            Provider::Anthropic => "anthropic",
            Provider::OpenAi => "openai",
            Provider::Zai => "zai",
            Provider::Compatible => "compatible",
        }
    }

    /// Libellé des Réglages (traduit à l'affichage).
    pub fn label(self) -> &'static str {
        match self {
            Provider::Lyra => "LYRA (abonnement de l'assistant)",
            Provider::Anthropic => "Anthropic (Claude)",
            Provider::OpenAi => "OpenAI",
            Provider::Zai => "Z.ai (GLM)",
            Provider::Compatible => "Compatible OpenAI (Ollama, LM Studio…)",
        }
    }

    pub fn default_endpoint(self) -> &'static str {
        match self {
            Provider::Lyra => "",
            Provider::Anthropic => "https://api.anthropic.com/v1/messages",
            Provider::OpenAi => "https://api.openai.com/v1/chat/completions",
            Provider::Zai => "https://api.z.ai/api/paas/v4/chat/completions",
            Provider::Compatible => "http://localhost:11434/v1/chat/completions",
        }
    }

    pub fn default_model(self) -> &'static str {
        match self {
            Provider::Lyra => "",
            Provider::Anthropic => "claude-opus-5-5",
            Provider::OpenAi => "gpt-5",
            Provider::Zai => "glm-4.6",
            Provider::Compatible => "",
        }
    }

    /// Variable d'environnement lue quand le trousseau n'a pas de clé.
    pub fn env_key(self) -> Option<&'static str> {
        match self {
            Provider::Anthropic => Some("ANTHROPIC_API_KEY"),
            Provider::OpenAi => Some("OPENAI_API_KEY"),
            Provider::Zai => Some("ZAI_API_KEY"),
            Provider::Lyra | Provider::Compatible => None,
        }
    }

    /// Le service s'appelle par HTTP (endpoint, modèle, clé).
    pub fn http(self) -> bool {
        self != Provider::Lyra
    }

    /// Sans clé, le service refuse (un service local compatible s'en passe).
    pub fn needs_key(self) -> bool {
        matches!(self, Provider::Anthropic | Provider::OpenAi | Provider::Zai)
    }
}

/// Identifiant du fournisseur intégré.
pub const LYRA_ID: &str = "lyra";

/// Un fournisseur configuré ; endpoint et modèle vides : ceux de son type.
#[derive(Debug, Clone, PartialEq)]
pub struct Service {
    /// Identifiant stable (clé du trousseau, `ai_use`).
    pub id: String,
    pub name: String,
    pub kind: Provider,
    pub endpoint: String,
    pub model: String,
}

impl Service {
    pub fn lyra() -> Self {
        Service {
            id: LYRA_ID.into(),
            name: "LYRA".into(),
            kind: Provider::Lyra,
            endpoint: String::new(),
            model: String::new(),
        }
    }

    pub fn endpoint(&self) -> String {
        let e = self.endpoint.trim();
        if e.is_empty() {
            self.kind.default_endpoint().to_string()
        } else {
            e.to_string()
        }
    }

    pub fn model(&self) -> String {
        let m = self.model.trim();
        if m.is_empty() {
            self.kind.default_model().to_string()
        } else {
            m.to_string()
        }
    }

    /// Nom affiché : le sien, sinon celui du type.
    pub fn label(&self) -> String {
        if self.name.trim().is_empty() {
            crate::i18n::tr(self.kind.label()).to_string()
        } else {
            self.name.trim().to_string()
        }
    }
}

/// Fournisseurs et fonction → fournisseur (`ai_services`, `ai_use`).
#[derive(Debug, Clone, PartialEq, Default)]
pub struct AiCfg {
    /// Fournisseurs ajoutés (LYRA, intégrée, n'y figure pas).
    pub services: Vec<Service>,
    pub uses: std::collections::BTreeMap<String, String>,
}

impl AiCfg {
    /// LYRA, puis les fournisseurs ajoutés.
    pub fn all(&self) -> Vec<Service> {
        std::iter::once(Service::lyra())
            .chain(self.services.iter().cloned())
            .collect()
    }

    pub fn service(&self, id: &str) -> Option<Service> {
        self.all().into_iter().find(|s| s.id == id)
    }

    /// Fournisseur d'une fonction : celui choisi, sinon LYRA (aussi quand le
    /// fournisseur choisi a été retiré).
    pub fn for_consumer(&self, consumer: &str) -> Service {
        self.uses
            .get(consumer)
            .and_then(|id| self.service(id))
            .unwrap_or_else(Service::lyra)
    }

    /// Identifiant libre pour un nouveau fournisseur du type donné.
    pub fn new_id(&self, kind: Provider) -> String {
        let base = kind.as_str();
        let taken = |id: &str| id == LYRA_ID || self.services.iter().any(|s| s.id == id);
        if !taken(base) {
            return base.to_string();
        }
        (2..)
            .map(|n| format!("{base}-{n}"))
            .find(|id| !taken(id))
            .unwrap_or_default()
    }
}

/// Identifiant valable : minuscules, chiffres, `-` et `_`.
pub fn valid_id(id: &str) -> bool {
    !id.is_empty()
        && id
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-' || c == '_')
}

/// Fonction de hyprdesk qui fait appel à un fournisseur.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Consumer {
    pub id: &'static str,
    /// Libellé et description (traduits à l'affichage).
    pub label: &'static str,
    pub summary: &'static str,
}

/// Fonctions servies par un fournisseur, dans l'ordre des Réglages.
pub const CONSUMERS: &[Consumer] = &[Consumer {
    id: "smartui",
    label: "SmartUI design",
    summary: "Icônes du lanceur redessinées dans un même style",
}];

/// D'où vient la clé du fournisseur.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum KeySource {
    Keyring,
    Env(&'static str),
    Missing,
}

fn keyring_attrs(id: &str) -> [&str; 4] {
    ["service", KEYRING_SERVICE, "provider", id]
}

fn keyring_lookup(id: &str) -> Option<String> {
    let out = Command::new("secret-tool")
        .arg("lookup")
        .args(keyring_attrs(id))
        .stdin(Stdio::null())
        .stderr(Stdio::null())
        .output()
        .ok()?;
    let k = String::from_utf8(out.stdout).ok()?.trim().to_string();
    (out.status.success() && !k.is_empty()).then_some(k)
}

/// Clé API du fournisseur : le trousseau, sinon la variable d'environnement de son type.
pub fn key(s: &Service) -> Option<String> {
    keyring_lookup(&s.id).or_else(|| {
        s.kind
            .env_key()
            .and_then(|v| std::env::var(v).ok())
            .map(|k| k.trim().to_string())
            .filter(|k| !k.is_empty())
    })
}

pub fn key_source(s: &Service) -> KeySource {
    if !s.kind.http() {
        return KeySource::Missing;
    }
    if keyring_lookup(&s.id).is_some() {
        return KeySource::Keyring;
    }
    match s.kind.env_key() {
        Some(v) if std::env::var(v).is_ok_and(|k| !k.trim().is_empty()) => KeySource::Env(v),
        _ => KeySource::Missing,
    }
}

/// Range la clé dans le trousseau (remplace la précédente).
pub fn store_key(id: &str, key: &str) -> Result<(), String> {
    let key = key.trim();
    if key.is_empty() {
        return Err(crate::i18n::tr("clé vide").into());
    }
    let mut child = Command::new("secret-tool")
        .arg("store")
        .arg(format!("--label=hyprdesk : clé API {id}"))
        .args(keyring_attrs(id))
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| {
            crate::tr!(
                "secret-tool introuvable (paquet libsecret) : {e}",
                e = e.to_string()
            )
        })?;
    if let Some(mut stdin) = child.stdin.take() {
        stdin.write_all(key.as_bytes()).map_err(|e| e.to_string())?;
    }
    let out = child.wait_with_output().map_err(|e| e.to_string())?;
    if out.status.success() {
        Ok(())
    } else {
        Err(crate::tr!(
            "trousseau : {e}",
            e = String::from_utf8_lossy(&out.stderr).trim().to_string()
        ))
    }
}

pub fn clear_key(id: &str) -> Result<(), String> {
    let out = Command::new("secret-tool")
        .arg("clear")
        .args(keyring_attrs(id))
        .output()
        .map_err(|e| e.to_string())?;
    if out.status.success() {
        Ok(())
    } else {
        Err(String::from_utf8_lossy(&out.stderr).trim().to_string())
    }
}

/// Fichier joint à la demande.
#[derive(Debug, Clone)]
pub enum Attachment {
    /// Image matricielle, envoyée telle quelle au modèle (vision).
    Png { path: PathBuf, bytes: Vec<u8> },
}

/// Une demande ponctuelle : consignes, message, fichier joint éventuel.
#[derive(Debug, Clone)]
pub struct Request {
    pub system: String,
    pub prompt: String,
    pub attachment: Option<Attachment>,
}

/// Envoie la demande de la fonction `consumer` à son fournisseur et rend le
/// texte de la réponse (bloquant : à appeler hors du fil de l'interface).
pub fn ask(cfg: &crate::config::Config, consumer: &str, req: &Request) -> Result<String, String> {
    let svc = cfg.general.ai.for_consumer(consumer);
    match svc.kind {
        Provider::Lyra => ask_lyra(cfg, req),
        Provider::Anthropic => ask_anthropic(&svc, req),
        _ => ask_openai(&svc, req),
    }
}

fn ask_lyra(cfg: &crate::config::Config, req: &Request) -> Result<String, String> {
    let bin = crate::assistant::lyra().ok_or_else(|| {
        crate::i18n::tr("LYRA absente : choisissez un autre service, ou installez LYRA").to_string()
    })?;
    let mut cmd = Command::new(bin);
    cmd.args(["cli", "--no-fs", "--no-color"]);
    if crate::assistant::supports_provider()
        && let Some(p) = crate::assistant::provider(cfg.general.apps.get("ai").map(String::as_str))
    {
        cmd.args(["--provider", p]);
    }
    if let Some(Attachment::Png { path, .. }) = &req.attachment {
        cmd.arg("-f").arg(path);
    }
    // Le message arrive sur l'entrée standard : une seule passe, sans session.
    let message = format!("{}\n\n{}", req.system, req.prompt);
    let (code, out, err) = run(cmd, Some(message.as_bytes()))?;
    let out = strip_ansi(&out);
    if code != Some(0) && !out.contains("<svg") {
        let why = strip_ansi(&err);
        let why = why.trim();
        return Err(crate::tr!(
            "LYRA : {e}",
            e = if why.is_empty() {
                out.trim().chars().take(300).collect::<String>()
            } else {
                why.chars().take(300).collect()
            }
        ));
    }
    Ok(out)
}

/// Familles qui acceptent le repli côté serveur (`fallbacks: "default"`).
fn anthropic_fallbacks(model: &str) -> bool {
    [
        "claude-fable-5-2",
        "claude-fable-5-1",
        "claude-opus-5-5",
        "claude-opus-5",
        "claude-sonnet-5-5",
    ]
    .contains(&model)
}

fn ask_anthropic(svc: &Service, req: &Request) -> Result<String, String> {
    let key = key(svc).ok_or_else(|| missing_key(svc))?;
    let endpoint = svc.endpoint();
    let model = svc.model();
    let mut content = Vec::new();
    if let Some(Attachment::Png { bytes, .. }) = &req.attachment {
        content.push(json!({
            "type": "image",
            "source": { "type": "base64", "media_type": "image/png", "data": base64(bytes) },
        }));
    }
    content.push(json!({ "type": "text", "text": req.prompt }));
    let mut body = json!({
        "model": model,
        "max_tokens": 16000,
        "system": req.system,
        "messages": [{ "role": "user", "content": content }],
    });
    let mut headers = vec![
        format!("x-api-key: {key}"),
        format!("anthropic-version: {ANTHROPIC_VERSION}"),
    ];
    // Refus d'un filtre de sécurité : le serveur rejoue la demande sur un
    // modèle de repli (endpoint officiel seulement, un proxy peut l'ignorer).
    if anthropic_fallbacks(&model) && endpoint == Provider::Anthropic.default_endpoint() {
        body["fallbacks"] = json!("default");
        headers.push("anthropic-beta: server-side-fallback-2026-07-01".into());
    }
    let v = post(&endpoint, &headers, &body)?;
    if v["stop_reason"] == "refusal" {
        return Err(crate::i18n::tr("le modèle a refusé la demande").into());
    }
    let text: String = v["content"]
        .as_array()
        .into_iter()
        .flatten()
        .filter(|b| b["type"] == "text")
        .filter_map(|b| b["text"].as_str())
        .collect::<Vec<_>>()
        .join("");
    if text.trim().is_empty() {
        return Err(crate::i18n::tr("réponse vide").into());
    }
    Ok(text)
}

fn ask_openai(svc: &Service, req: &Request) -> Result<String, String> {
    let key = key(svc);
    if key.is_none() && svc.kind.needs_key() {
        return Err(missing_key(svc));
    }
    let model = svc.model();
    if model.is_empty() {
        return Err(
            crate::i18n::tr("modèle à préciser (Réglages → IA → Autres services IA)").into(),
        );
    }
    let user = match &req.attachment {
        Some(Attachment::Png { bytes, .. }) => json!([
            { "type": "text", "text": req.prompt },
            { "type": "image_url", "image_url": { "url": format!("data:image/png;base64,{}", base64(bytes)) } },
        ]),
        None => json!(req.prompt),
    };
    let body = json!({
        "model": model,
        "messages": [
            { "role": "system", "content": req.system },
            { "role": "user", "content": user },
        ],
    });
    let headers: Vec<String> = key
        .map(|k| format!("authorization: Bearer {k}"))
        .into_iter()
        .collect();
    let v = post(&svc.endpoint(), &headers, &body)?;
    let msg = &v["choices"][0]["message"]["content"];
    let text = match msg {
        Value::String(s) => s.clone(),
        // Certains services rendent des morceaux typés.
        Value::Array(parts) => parts
            .iter()
            .filter_map(|p| p["text"].as_str())
            .collect::<Vec<_>>()
            .join(""),
        _ => String::new(),
    };
    if text.trim().is_empty() {
        return Err(crate::i18n::tr("réponse vide").into());
    }
    Ok(text)
}

fn missing_key(svc: &Service) -> String {
    crate::tr!(
        "{name} : aucune clé API (Réglages → IA → Autres services IA)",
        name = svc.label()
    )
}

/// POST JSON par curl : la configuration (URL, en-têtes, dont la clé) sur
/// l'entrée standard, le corps dans un fichier privé.
fn post(url: &str, headers: &[String], body: &Value) -> Result<Value, String> {
    if !(url.starts_with("https://") || url.starts_with("http://")) {
        return Err(crate::tr!(
            "endpoint invalide : {url}",
            url = url.to_string()
        ));
    }
    let dir = cache_dir();
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let file = dir.join(format!(
        "requete-{}-{}.json",
        std::process::id(),
        rand::random::<u32>()
    ));
    write_private(&file, body.to_string().as_bytes()).map_err(|e| e.to_string())?;
    let mut conf = format!("url = {}\n", curl_quote(url));
    conf.push_str("header = \"content-type: application/json\"\n");
    for h in headers {
        conf.push_str(&format!("header = {}\n", curl_quote(h)));
    }
    conf.push_str(&format!(
        "data-binary = {}\n",
        curl_quote(&format!("@{}", file.display()))
    ));
    let mut cmd = Command::new("curl");
    cmd.args(["-sS", "--max-time", &TIMEOUT.as_secs().to_string()])
        .args(["-w", "\n%{http_code}", "-K", "-"]);
    let result = run(cmd, Some(conf.as_bytes()));
    let _ = std::fs::remove_file(&file);
    let (code, out, err) = result?;
    if code != Some(0) {
        return Err(crate::tr!("curl : {e}", e = err.trim().to_string()));
    }
    let (payload, status) = out.rsplit_once('\n').unwrap_or((out.as_str(), ""));
    let status: u16 = status.trim().parse().unwrap_or(0);
    let v: Value = serde_json::from_str(payload).unwrap_or(Value::Null);
    if !(200..300).contains(&status) {
        let why = v["error"]["message"]
            .as_str()
            .or(v["error"].as_str())
            .or(v["message"].as_str())
            .map(String::from)
            .unwrap_or_else(|| payload.chars().take(200).collect());
        return Err(crate::tr!(
            "HTTP {status} : {why}",
            status = status.to_string(),
            why = why
        ));
    }
    if v.is_null() {
        return Err(crate::i18n::tr("réponse illisible").into());
    }
    Ok(v)
}

/// Lance la commande, avec une entrée et un délai ; rend (code, sortie, erreurs).
fn run(mut cmd: Command, input: Option<&[u8]>) -> Result<(Option<i32>, String, String), String> {
    let mut child = cmd
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| e.to_string())?;
    // L'entrée se ferme en sortant du bloc : le programme voit la fin.
    if let Some(mut stdin) = child.stdin.take()
        && let Some(input) = input
    {
        let _ = stdin.write_all(input);
    }
    // Lecture en parallèle : un tuyau plein bloquerait le processus.
    let mut stdout = child.stdout.take();
    let mut stderr = child.stderr.take();
    let out = std::thread::spawn(move || {
        let mut s = String::new();
        if let Some(o) = stdout.as_mut() {
            let _ = std::io::Read::read_to_string(o, &mut s);
        }
        s
    });
    let err = std::thread::spawn(move || {
        let mut s = String::new();
        if let Some(e) = stderr.as_mut() {
            let _ = std::io::Read::read_to_string(e, &mut s);
        }
        s
    });
    let start = Instant::now();
    let status = loop {
        match child.try_wait().map_err(|e| e.to_string())? {
            Some(s) => break s,
            None if start.elapsed() > TIMEOUT + Duration::from_secs(10) => {
                let _ = child.kill();
                let _ = child.wait();
                return Err(crate::i18n::tr("délai dépassé").into());
            }
            None => std::thread::sleep(Duration::from_millis(100)),
        }
    };
    Ok((
        status.code(),
        out.join().unwrap_or_default(),
        err.join().unwrap_or_default(),
    ))
}

fn cache_dir() -> PathBuf {
    std::env::var_os("XDG_CACHE_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            PathBuf::from(std::env::var_os("HOME").unwrap_or_default()).join(".cache")
        })
        .join("hyprdesk")
}

fn write_private(path: &Path, bytes: &[u8]) -> std::io::Result<()> {
    use std::os::unix::fs::OpenOptionsExt as _;
    let mut f = std::fs::OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .mode(0o600)
        .open(path)?;
    f.write_all(bytes)
}

/// Valeur d'un fichier de configuration de curl, entre guillemets.
fn curl_quote(s: &str) -> String {
    let mut q = String::with_capacity(s.len() + 2);
    q.push('"');
    for c in s.chars() {
        match c {
            '\\' => q.push_str("\\\\"),
            '"' => q.push_str("\\\""),
            '\n' | '\r' => {}
            c => q.push(c),
        }
    }
    q.push('"');
    q
}

/// Séquences de couleur du terminal retirées (sortie de LYRA).
fn strip_ansi(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut chars = s.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '\x1b' {
            if chars.peek() == Some(&'[') {
                chars.next();
                for c in chars.by_ref() {
                    if c.is_ascii_alphabetic() {
                        break;
                    }
                }
            }
            continue;
        }
        out.push(c);
    }
    out
}

pub fn base64(bytes: &[u8]) -> String {
    const T: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity(bytes.len().div_ceil(3) * 4);
    for chunk in bytes.chunks(3) {
        let b = [
            chunk[0],
            *chunk.get(1).unwrap_or(&0),
            *chunk.get(2).unwrap_or(&0),
        ];
        let n = (u32::from(b[0]) << 16) | (u32::from(b[1]) << 8) | u32::from(b[2]);
        out.push(T[(n >> 18) as usize & 63] as char);
        out.push(T[(n >> 12) as usize & 63] as char);
        out.push(if chunk.len() > 1 {
            T[(n >> 6) as usize & 63] as char
        } else {
            '='
        });
        out.push(if chunk.len() > 2 {
            T[n as usize & 63] as char
        } else {
            '='
        });
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fournisseurs() {
        for p in PROVIDERS {
            assert_eq!(Provider::parse(p.as_str()), Some(p));
        }
        assert_eq!(Provider::parse("Claude"), Some(Provider::Anthropic));
        assert_eq!(Provider::parse("ollama"), Some(Provider::Compatible));
        assert_eq!(Provider::parse("inconnu"), None);
        let s = Service {
            id: "a".into(),
            name: String::new(),
            kind: Provider::Anthropic,
            endpoint: String::new(),
            model: String::new(),
        };
        assert_eq!(s.endpoint(), "https://api.anthropic.com/v1/messages");
        assert_eq!(s.model(), "claude-opus-5-5");
        let s = Service {
            endpoint: " https://proxy/v1 ".into(),
            model: "glm-4.5v".into(),
            kind: Provider::Zai,
            ..s
        };
        assert_eq!(
            (s.endpoint().as_str(), s.model().as_str()),
            ("https://proxy/v1", "glm-4.5v")
        );
    }

    /// Par défaut tout passe par LYRA, aussi quand le fournisseur choisi a disparu.
    #[test]
    fn fonction_vers_fournisseur() {
        let mut ai = AiCfg::default();
        assert_eq!(ai.for_consumer("smartui").id, LYRA_ID);
        assert_eq!(ai.new_id(Provider::Anthropic), "anthropic");
        ai.services.push(Service {
            id: "anthropic".into(),
            name: "Claude perso".into(),
            kind: Provider::Anthropic,
            endpoint: String::new(),
            model: String::new(),
        });
        assert_eq!(ai.new_id(Provider::Anthropic), "anthropic-2");
        ai.uses.insert("smartui".into(), "anthropic".into());
        assert_eq!(ai.for_consumer("smartui").label(), "Claude perso");
        ai.uses.insert("smartui".into(), "retire".into());
        assert_eq!(ai.for_consumer("smartui").id, LYRA_ID);
        assert_eq!(ai.all().len(), 2);
        assert!(valid_id("claude-2") && !valid_id("Claude") && !valid_id(""));
    }

    #[test]
    fn encodages() {
        assert_eq!(base64(b""), "");
        assert_eq!(base64(b"f"), "Zg==");
        assert_eq!(base64(b"fo"), "Zm8=");
        assert_eq!(base64(b"foobar"), "Zm9vYmFy");
        assert_eq!(curl_quote("a\"b\\c\nd"), "\"a\\\"b\\\\cd\"");
        assert_eq!(strip_ansi("\x1b[1;32m<svg/>\x1b[0m"), "<svg/>");
    }

    #[test]
    fn endpoint_refuse_sans_http() {
        let e = post("file:///etc/passwd", &[], &json!({})).unwrap_err();
        assert!(e.contains("file:///etc/passwd"), "{e}");
    }
}
