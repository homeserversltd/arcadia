use std::{env, net::SocketAddr, path::PathBuf};

#[derive(Debug, PartialEq, Eq)]
pub struct ServeConfig {
    pub http_bind: Option<SocketAddr>,
    pub https_bind: Option<SocketAddr>,
    pub tls_cert: Option<PathBuf>,
    pub tls_key: Option<PathBuf>,
}

impl ServeConfig {
    pub fn from_env_and_args(args: &[String]) -> Result<Self, String> {
        let http = value(args, "--http-bind")
            .or_else(|| env_value("ARCADIA_HTTP_BIND"))
            .or_else(|| env_value("ARCADIA_BIND"))
            .unwrap_or_else(|| "0.0.0.0:8080".to_string());
        let https = value(args, "--https-bind").or_else(|| env_value("ARCADIA_HTTPS_BIND"));
        let cert = value(args, "--tls-cert")
            .or_else(|| env_value("ARCADIA_TLS_CERT"))
            .map(PathBuf::from);
        let key = value(args, "--tls-key")
            .or_else(|| env_value("ARCADIA_TLS_KEY"))
            .map(PathBuf::from);

        let http_bind = parse_optional_addr(&http, "HTTP")?;
        let https_bind = match (https, cert.is_some() || key.is_some()) {
            (Some(bind), _) => Some(parse_addr(&bind, "HTTPS")?),
            (None, true) => Some("0.0.0.0:443".parse().expect("valid default HTTPS bind")),
            (None, false) => None,
        };
        if https_bind.is_some() && (cert.is_none() || key.is_none()) {
            return Err("HTTPS serving requires both ARCADIA_TLS_CERT/--tls-cert and ARCADIA_TLS_KEY/--tls-key".into());
        }
        Ok(Self {
            http_bind,
            https_bind,
            tls_cert: cert,
            tls_key: key,
        })
    }
}

fn env_value(name: &str) -> Option<String> {
    env::var(name).ok().filter(|value| !value.trim().is_empty())
}

fn value(args: &[String], flag: &str) -> Option<String> {
    args.windows(2)
        .find(|pair| pair[0] == flag)
        .map(|pair| pair[1].clone())
}

fn parse_addr(value: &str, label: &str) -> Result<SocketAddr, String> {
    value
        .parse()
        .map_err(|_| format!("invalid {label} bind address: {value}"))
}

fn parse_optional_addr(value: &str, label: &str) -> Result<Option<SocketAddr>, String> {
    if value.eq_ignore_ascii_case("disabled") || value.eq_ignore_ascii_case("none") {
        Ok(None)
    } else {
        parse_addr(value, label).map(Some)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tls_paths_enable_standard_https_without_embedding_credentials() {
        let args = [
            "--tls-cert".into(),
            "/run/arcadia/cert.pem".into(),
            "--tls-key".into(),
            "/run/arcadia/key.pem".into(),
        ];
        let config = ServeConfig::from_env_and_args(&args).unwrap();
        assert_eq!(config.https_bind, Some("0.0.0.0:443".parse().unwrap()));
        assert_eq!(
            config.tls_cert,
            Some(PathBuf::from("/run/arcadia/cert.pem"))
        );
        assert_eq!(config.tls_key, Some(PathBuf::from("/run/arcadia/key.pem")));
    }

    #[test]
    fn https_requires_both_certificate_and_key() {
        let args = ["--https-bind".into(), "127.0.0.1:8443".into()];
        let error = ServeConfig::from_env_and_args(&args).unwrap_err();
        assert!(error.contains("both"));
    }

    #[test]
    fn http_health_listener_can_be_disabled_for_tls_only_service() {
        let args = [
            "--http-bind".into(),
            "disabled".into(),
            "--https-bind".into(),
            "127.0.0.1:8443".into(),
            "--tls-cert".into(),
            "cert.pem".into(),
            "--tls-key".into(),
            "key.pem".into(),
        ];
        let config = ServeConfig::from_env_and_args(&args).unwrap();
        assert_eq!(config.http_bind, None);
        assert_eq!(config.https_bind, Some("127.0.0.1:8443".parse().unwrap()));
    }
}
