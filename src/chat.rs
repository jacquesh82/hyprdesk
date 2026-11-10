//! Protocole « serve » de LYRA (JSON Lines sur stdin/stdout) : hyprdesk
//! affiche la conversation dans son tiroir, LYRA garde tout le moteur
//! (fournisseurs, outils, confirmations, mémoire, périmètre, `context.d`).
//!
//! Une ligne = un objet JSON, distingué par `type`. Les champs inconnus sont
//! ignorés ; un `type` inconnu donne [`ServerMsg::Unknown`], sans erreur.
//!
//! `hyprdesk chat-mock` est un faux moteur qui parle ce protocole : il sert
//! au développement, aux tests et à l'aperçu, sans LYRA ni abonnement.

use std::io::{BufRead, Write};

use serde::{Deserialize, Serialize};

/// Version du protocole parlée par hyprdesk.
pub const PROTOCOL: u32 = 1;

/// Pièce jointe d'un message.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Attachment {
    pub path: String,
}

/// Messages de hyprdesk vers le moteur.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ClientMsg {
    Hello {
        client: String,
        protocol: u32,
        language: String,
    },
    Message {
        id: String,
        text: String,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        attachments: Vec<Attachment>,
    },
    Confirm {
        request: String,
        accept: bool,
    },
    Choice {
        request: String,
        value: String,
    },
    Cancel,
    NewConversation,
    Provider {
        value: String,
    },
}

/// Option d'un choix proposé par le moteur.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ChoiceOption {
    pub label: String,
    pub value: String,
    #[serde(default)]
    pub preview: Option<String>,
}

/// Messages du moteur vers hyprdesk.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ServerMsg {
    Ready {
        protocol: u32,
        provider: String,
        #[serde(default)]
        providers: Vec<String>,
        #[serde(default)]
        fs: bool,
    },
    TurnStart {
        id: String,
    },
    Delta {
        text: String,
    },
    Thinking {
        active: bool,
    },
    Progress {
        phase: String,
        #[serde(default)]
        fraction: Option<f32>,
    },
    Tool {
        id: String,
        name: String,
        #[serde(default)]
        summary: String,
        status: String,
        #[serde(default)]
        result: Option<String>,
    },
    ConfirmRequest {
        id: String,
        command: String,
        #[serde(default)]
        reason: String,
        #[serde(default)]
        risk: String,
    },
    ChoiceRequest {
        id: String,
        #[serde(default)]
        kind: String,
        #[serde(default)]
        prompt: String,
        #[serde(default)]
        options: Vec<ChoiceOption>,
    },
    TurnEnd {
        id: String,
        #[serde(default)]
        text: String,
    },
    Error {
        message: String,
        #[serde(default)]
        fatal: bool,
    },
    /// Type inconnu (version plus récente du moteur) : ignoré.
    #[serde(other)]
    Unknown,
}

/// Une ligne JSON → message du moteur ; une ligne illisible donne une erreur non fatale.
pub fn parse(line: &str) -> ServerMsg {
    serde_json::from_str(line).unwrap_or_else(|e| ServerMsg::Error {
        message: format!("ligne illisible du moteur : {e}"),
        fatal: false,
    })
}

/// Message → ligne JSON (sans retour à la ligne).
pub fn encode(m: &ClientMsg) -> String {
    serde_json::to_string(m).unwrap_or_default()
}

/// Faux moteur (`hyprdesk chat-mock`) : répond en flux, montre un outil, une
/// confirmation et un choix. Aucun accès réseau, aucune commande exécutée.
pub fn mock() -> std::process::ExitCode {
    let stdin = std::io::stdin();
    let mut out = std::io::stdout().lock();
    let mut send = |m: &ServerMsg| {
        let _ = writeln!(out, "{}", serde_json::to_string(m).unwrap_or_default());
        let _ = out.flush();
    };
    let mut provider = "claude_code".to_string();
    let pause = || std::thread::sleep(std::time::Duration::from_millis(60));
    for line in stdin.lock().lines() {
        let Ok(line) = line else { break };
        let Ok(msg) = serde_json::from_str::<ClientMsg>(&line) else {
            send(&ServerMsg::Error {
                message: "message illisible".into(),
                fatal: false,
            });
            continue;
        };
        match msg {
            ClientMsg::Hello { .. } => send(&ServerMsg::Ready {
                protocol: PROTOCOL,
                provider: provider.clone(),
                providers: vec!["zcode".into(), "claude_code".into(), "codex".into()],
                fs: false,
            }),
            ClientMsg::Provider { value } => provider = value,
            ClientMsg::Message { id, text, .. } => {
                send(&ServerMsg::TurnStart { id: id.clone() });
                send(&ServerMsg::Thinking { active: true });
                pause();
                send(&ServerMsg::Thinking { active: false });
                let lower = text.to_lowercase();
                if lower.contains("gaps") {
                    send(&ServerMsg::Tool {
                        id: "t1".into(),
                        name: "hypr_get_option".into(),
                        summary: "Lecture de general:gaps_out".into(),
                        status: "done".into(),
                        result: Some("5 5 5 5".into()),
                    });
                    send(&ServerMsg::ConfirmRequest {
                        id: "c1".into(),
                        command: "hyprctl keyword general:gaps_out 10".into(),
                        reason: "Passer les marges extérieures à 10 px (aperçu, non enregistré)."
                            .into(),
                        risk: "low".into(),
                    });
                    continue;
                }
                if lower.contains("couleur") {
                    send(&ServerMsg::ChoiceRequest {
                        id: "q1".into(),
                        kind: "color".into(),
                        prompt: "Quelle couleur d'accent ?".into(),
                        options: ["#9580ff", "#ff80bf", "#80ffea"]
                            .iter()
                            .map(|c| ChoiceOption {
                                label: c.to_string(),
                                value: c.to_string(),
                                preview: Some(c.to_string()),
                            })
                            .collect(),
                    });
                    continue;
                }
                let answer = format!(
                    "Voici une **réponse de démonstration** ({provider}).\n\n\
                     - le texte arrive *au fil de l'eau* ;\n\
                     - le Markdown est rendu (`code`, listes, liens).\n\n\
                     Vous avez écrit : « {text} »."
                );
                for chunk in answer.split_inclusive(' ') {
                    send(&ServerMsg::Delta {
                        text: chunk.to_string(),
                    });
                    std::thread::sleep(std::time::Duration::from_millis(15));
                }
                send(&ServerMsg::TurnEnd { id, text: answer });
            }
            ClientMsg::Confirm { request, accept } => {
                let text = if accept {
                    "C'est fait : `gaps_out` vaut maintenant 10.".to_string()
                } else {
                    format!("D'accord, je n'ai rien changé ({request} refusée).")
                };
                send(&ServerMsg::Delta { text: text.clone() });
                send(&ServerMsg::TurnEnd { id: request, text });
            }
            ClientMsg::Choice { request, value } => {
                let text = format!("Couleur retenue : **{value}**.");
                send(&ServerMsg::Delta { text: text.clone() });
                send(&ServerMsg::TurnEnd { id: request, text });
            }
            ClientMsg::Cancel => send(&ServerMsg::TurnEnd {
                id: String::new(),
                text: "(interrompu)".into(),
            }),
            ClientMsg::NewConversation => {}
        }
    }
    std::process::ExitCode::SUCCESS
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn protocole() {
        // Côté moteur : champs connus, inconnus ignorés, type inconnu toléré.
        assert_eq!(
            parse(r#"{"type":"delta","text":"Bon","extra":1}"#),
            ServerMsg::Delta { text: "Bon".into() }
        );
        assert_eq!(
            parse(r#"{"type":"nouveau_type","x":1}"#),
            ServerMsg::Unknown
        );
        assert!(matches!(
            parse(r#"{"type":"confirm_request","id":"c1","command":"ls"}"#),
            ServerMsg::ConfirmRequest { id, .. } if id == "c1"
        ));
        assert!(matches!(
            parse("pas du json"),
            ServerMsg::Error { fatal: false, .. }
        ));
        // Côté client : la forme attendue par LYRA.
        assert_eq!(
            encode(&ClientMsg::Confirm {
                request: "c1".into(),
                accept: true
            }),
            r#"{"type":"confirm","request":"c1","accept":true}"#
        );
        assert_eq!(encode(&ClientMsg::Cancel), r#"{"type":"cancel"}"#);
        assert_eq!(
            encode(&ClientMsg::Message {
                id: "m1".into(),
                text: "hé".into(),
                attachments: vec![]
            }),
            r#"{"type":"message","id":"m1","text":"hé"}"#
        );
    }
}
