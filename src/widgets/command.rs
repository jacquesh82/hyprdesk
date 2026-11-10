//! Widget « command » : la sortie d'une commande shell, relancée à intervalle.

use std::time::Duration;

use super::Value;

pub async fn run_command(cmd: &str, timeout: Duration) -> Value {
    match crate::processes::output(
        tokio::process::Command::new("sh").args(["-c", cmd]),
        timeout,
    )
    .await
    {
        Err(e) => Value::Error(e.to_string()),
        Ok(o) => {
            let out = String::from_utf8_lossy(&o.stdout);
            // Au plus 20 lignes : un widget n'est pas un terminal.
            let text: Vec<&str> = out.trim_end().lines().take(20).collect();
            if !o.status.success() && text.is_empty() {
                let err = String::from_utf8_lossy(&o.stderr);
                Value::Error(
                    err.lines()
                        .next()
                        .unwrap_or(crate::i18n::tr("échec"))
                        .to_string(),
                )
            } else {
                Value::Text(text.join("\n"))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn commande_et_delai() {
        assert_eq!(
            run_command("printf 'a\\nb'", Duration::from_secs(5)).await,
            Value::Text("a\nb".into())
        );
        assert!(matches!(
            run_command("sleep 5", Duration::from_millis(100)).await,
            Value::Error(_)
        ));
    }
}
