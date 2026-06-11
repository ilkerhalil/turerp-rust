# HTTPS Setup (Caddy + Let's Encrypt)

Single-host HTTPS termination for the turerp-rust API. The pattern:
**Caddy in front, app behind, no TLS inside docker network.**

## Why this design

- **Caddy auto-issues and auto-renews** Let's Encrypt certs. Zero
  shell scripts, no certbot cron.
- **App stays unchanged** — it speaks plain HTTP on `0.0.0.0:8080`
  inside the docker network. Caddy terminates TLS and forwards
  plaintext. This is the standard 3-tier pattern; nginx and traefik
  do the same.
- **Single host, single cert** — the Caddyfile is one block, the
  compose overlay is one file. No service mesh, no ingress
  controller, no DNS provider integration.
- **Healthcheck-aware startup** — `caddy` waits for `turerp` to be
  healthy before accepting traffic. Compose dependency + healthcheck
  chain.

## When NOT to use this

- **Multi-host / k8s** — Caddy still works but you'll likely want an
  ingress controller with cert-manager. Same pattern, different
  machinery.
- **Wildcard certs** — Caddy only does HTTP-01 here. If you need a
  wildcard (`*.turerp.example.com`), switch to DNS-01 with your
  provider's ACME plugin. Out of scope for the 2-week minimal cut.
- **mTLS to upstream** — app is HTTP on the internal network. If
  you need TLS to the app, front it with a second Caddy or a sidecar.

## One-time host prep

```bash
# 1. DNS A record
#    api.turerp.example.com  →  <public IP of this host>
#
# 2. Outbound 80/443 must be reachable from this host (LE challenge
#    comes IN on 80, cert responses go OUT on 443).
#
# 3. Host firewall: allow 80/tcp and 443/tcp. Nothing else needs
#    to be open to the public internet — 8080 stays on the docker
#    bridge.

sudo ufw allow 80/tcp
sudo ufw allow 443/tcp
```

## Deploy

```bash
cd /opt/turerp   # or wherever the repo lives

# 1. Pull the latest main
git pull --rebase

# 2. Set the two required env vars
export TURERP_DOMAIN=api.turerp.example.com
export TURERP_ACME_EMAIL=ops@turerp.example.com

# 3. Bring up the base stack + the HTTPS overlay
docker compose -f docker-compose.yml -f docker-compose.https.yml up -d

# 4. Watch Caddy issue the cert (first run takes 30-60s)
docker compose -f docker-compose.yml -f docker-compose.https.yml logs -f caddy
# Look for: "obtained certificate"  and  "serving HTTPS on :443"
```

## Verify

```bash
# 1. Cert chain
openssl s_client -connect $TURERP_DOMAIN:443 -servername $TURERP_DOMAIN < /dev/null 2>/dev/null \
    | openssl x509 -noout -issuer -subject -dates

# Expected issuer: "Let's Encrypt" (or "R3"/"R10" intermediate)
# Expected notAfter: 90 days from now

# 2. HTTP → HTTPS redirect
curl -sI http://$TURERP_DOMAIN/health/live | head -3
# Expected: HTTP/1.1 308 Permanent Redirect, Location: https://...

# 3. Full round-trip via hurl
cd turerp/tests/hurl
BASE_URL=https://$TURERP_DOMAIN TURERP_TEST_PASSWORD='...' ./run-all.sh
# Expected: 22/22 passed, 0 failed
```

## Renewal

Caddy renews automatically when the cert is <30 days from expiry.
The `caddy_data` volume persists the issued cert + ACME account
key, so a container restart does NOT trigger re-issuance.

If renewal fails (e.g. DNS record was removed), Caddy logs a warning
and keeps serving the old cert until it expires. To force
re-issuance:

```bash
docker compose -f docker-compose.yml -f docker-compose.https.yml restart caddy
```

## Roll back to HTTP-only

```bash
docker compose -f docker-compose.yml -f docker-compose.https.yml down caddy
# Base compose continues running with port 8080 exposed. The hurl
# suite can be re-run with BASE_URL=http://... to verify.
```

## Adding a second domain (SPA, admin UI)

Edit `Caddyfile` and add a new `reverse_proxy` block with the same
upstream, or use a `route` block to match by Host. Caddy will
auto-issue a SAN cert covering both names.

## Operational notes

- **LE rate limit** — 5 certs per week per registered domain. Don't
  churn the `TURERP_DOMAIN` value; the `caddy_data` volume holds
  the issued cert.
- **Staging LE** — for testing the deploy without burning the
  rate limit, swap the Caddy image to `caddy:2-builder` and run
  `caddy adapt` with the staging CA endpoint. Out of scope for
  the 2-week minimal cut.
- **HSTS preload** — the Caddyfile deliberately does NOT enable
  HSTS preload (no `header Strict-Transport-Security "max-age=63072000; preload"`).
  Preload is a one-way door that takes months to undo if you
  misconfigure TLS. Operators can add it after the first
  successful production week, once the cert chain is stable.
