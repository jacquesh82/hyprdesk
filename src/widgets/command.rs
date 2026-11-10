//! Widget « command » : la sortie d'une commande shell, relancée à intervalle.

use std::time::Duration;

use super::Value;

pub async fn run_command(cmd: &str, timeout: Duration) -> Value {
    let child = tokio::process::Command::new("sh").arg("-c").arg(cmd).kill_on_drop(true).output();
    match tokio::time::timeout(timeout, child).await {
        Err(_) => Value::Error(format!("délai dépassé ({} s)", timeout.as_secs_f32())),
        Ok(Err(e)) => Value::Error(e.to_string()),
        Ok(Ok(o)) => {
            let out = String::from_utf8_lossy(&o.stdout);
            // Au plus 20 lignes : un widget n'est pas un terminal.
            let text: Vec<&str> = out.trim_end().lines().take(20).collect();
            if !o.status.success() && text.is_empty() {
                let err = String::from_utf8_lossy(&o.stderr);
                Value::Error(err.lines().next().unwrap_or("échec").to_string())
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
        assert_eq!(run_command("printf 'a\\nb'", Duration::from_secs(5)).await, Value::Text("a\nb".into()));
        assert!(matches!(run_command("sleep 5", Duration::from_millis(100)).await, Value::Error(_)));
    }
}
