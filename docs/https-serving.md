# Native HTTPS serving

Arcadia serves HTTPS directly with Rustls. It does not generate, embed, or
skip verification for certificates. The certificate file must contain the
server certificate (and any required intermediates); the key file must be
readable by the service account and must remain outside the repository.

## Configuration

HTTPS is enabled when both `ARCADIA_TLS_CERT` and `ARCADIA_TLS_KEY` are set.
It defaults to `0.0.0.0:443`; set `ARCADIA_HTTPS_BIND` to use another address.
The equivalent command-line options are `--tls-cert`, `--tls-key`, and
`--https-bind`, and take precedence over environment variables.

The existing HTTP health listener remains enabled at `ARCADIA_BIND` (or
`ARCADIA_HTTP_BIND`), defaulting to `0.0.0.0:8080`. Set either HTTP variable to
`disabled` for HTTPS-only operation. Keeping HTTP enabled is useful for local
health checks; it is not an HTTP-to-HTTPS redirect and should be firewalled if
the deployment requires HTTPS-only access.

When HTTPS is configured and `ARCADIA_CANONICAL_URL` is unset, the UI advertises
`https://console.home.arpa/`. Set `ARCADIA_CANONICAL_URL` when the deployment
uses another hostname.

## systemd with an unprivileged service account

Binding port 443 does not require running Arcadia as root. Grant only
`CAP_NET_BIND_SERVICE` to the service and keep the private key readable only by
the `owner` account (or use a tightly scoped credential/provisioning mechanism):

```ini
[Service]
User=owner
ExecStart=/opt/arcadia/arcadia --http-bind disabled --https-bind 0.0.0.0:443 \
  --tls-cert /etc/arcadia/tls/console.home.arpa.fullchain.pem \
  --tls-key /etc/arcadia/tls/console.home.arpa.key
AmbientCapabilities=CAP_NET_BIND_SERVICE
CapabilityBoundingSet=CAP_NET_BIND_SERVICE
NoNewPrivileges=true
```

The service manager must also grant `owner` read access to the certificate and
key paths. Use the normal system trust configuration for clients; Arcadia does
not provide a verification bypass.