# 81_observability_write — WRITE-FENCE for observability SLI/SLO/alert mutations.
#
# Covers the admin-gated mutations the read-only 46_observability.hurl never
# exercises:
#   /observability/slis                      POST (create SLI)
#   /observability/slis/{sli_id}/measurements POST (record measurement, raw f64 body)
#   /observability/slos                       POST (create SLO, references sli_id)
#   /observability/slos/compliance/evaluate   POST (evaluate SLO compliance)
#   /observability/alert-rules                POST (create) + DELETE /{id}
#   /observability/alerts/evaluate           POST (evaluate rules w/ metrics map)
#   /observability/alerts/{id}/resolve       POST (resolve a fired alert)
#
# Why: 46 only hits the open list endpoints (slis/slos/alert-rules/alerts/
# dashboard, all 200). The create/record/evaluate/resolve decode paths
# (SliDefinition with SliMetricType enum, SloDefinition with SloTarget enum,
# AlertRule with AlertSeverity enum, Alert with AlertState enum) stay hidden
# until a real SLI/SLO/alert-rule is seeded.
#
# FK STRATEGY: self-seed an SLI (admin) → capture sli_id (String); use it as
# the sli_id FK when creating the SLO. Self-seed an alert rule → capture
# rule_id; evaluate_alerts with a metrics map that trips the rule → capture
# the fired alert id → resolve it. All ids are String (UUID-ish), captured
# via jsonpath "$.id".
#
# RBAC: create_sli / record_sli / create_slo / evaluate_slo_compliance /
# create_alert_rule / delete_alert_rule / evaluate_alerts / resolve_alert all
# use AdminUser → testuser ({{access_token}}) gets 403, admin ({{admin_token}})
# gets 200/201/204. list endpoints (slis/slos/alert-rules/alerts/compliance/
# dashboard/sparklines) take NO auth extractor → open to anyone (no 401).
# run-all.sh SKIPS this file when admin_token is empty.
#
# ASSUMPTION: SliMetricType / AlertSeverity / SloTarget / SloStatus /
# AlertState have NO #[serde(rename_all)] → serde serializes variant names
# in PascalCase: "Availability", "Latency", "ErrorRate", ...; "Info"/
# "Warning"/"Critical"; "Gte"/"Lte"; "Compliant"/"AtRisk"/"Breached";
# "Firing"/"Resolved"/"Silenced". The lowercase Display impls are NOT serde.
# Verified against src/domain/observability/model.rs.
# ASSUMPTION: record_sli takes a raw f64 body (web::Json<f64>), NOT an
# object — the request body is a bare JSON number (e.g. 0.99).
# ASSUMPTION: sli_id / alert rule id / alert id are String (UUID) returned by
# the in-memory service; capture as string and interpolate in paths.
# ASSUMPTION: evaluate_alert_rules matches condition as a lowercase short
# token ("gt"/"gte"/"lt"/"lte"/"eq") — NOT the symbolic ">" (the service does
# `match rule.condition.as_str() { "gt" => ... }`). Sending ">" would never
# trigger and step 9 capture would fail. We send "gt".
# ASSUMPTION: with duration_sec > 0 the alert tracker requires the condition
# to be sustained for duration_sec before firing (first observation returns
# None). We set duration_sec=0 so the rule fires immediately on the first
# evaluate when the metric trips it (cpu_usage=95 > threshold 80).
# ASSUMPTION: evaluate_alerts returns >=1 Alert when the metrics map trips a
# rule. Alert ids are format "alert-<N>" (deterministic counter). If no alert
# fires, step 9 capture fails — flagged for review.
# ASSUMPTION: ids are unique per run (UUID-generated), so re-runs won't 409.
# The service is in-memory (no DB persistence across restarts), so a
# freshly restarted server has empty SLI/SLO/alert-rule tables; this fence
# self-seeds everything it needs.
# ASSUMPTION: list endpoints are open (no auth extractor) per the source;
# no-auth GET /slis would return 200, so this fence does NOT assert 401 on
# the list endpoints — only the admin-gated mutations get the 401/403 fence.

# --- 1. POST SLI (admin) → 201 SliDefinition; capture sli_id ---
POST {{base_url}}/api/v1/observability/slis
Authorization: Bearer {{admin_token}}
Content-Type: application/json
```
{
  "name": "hurl-api-availability",
  "metric_type": "Availability",
  "source": "prometheus",
  "window_minutes": 5
}
```
HTTP 201
[Captures]
sli_id: jsonpath "$.id"
[Asserts]
jsonpath "$.id" isString
jsonpath "$.name" == "hurl-api-availability"
jsonpath "$.metric_type" == "Availability"
jsonpath "$.source" == "prometheus"
jsonpath "$.window_minutes" == 5
jsonpath "$.created_at" exists

# --- 2. POST measurement (admin) → 204; raw f64 body (NOT an object) ---
POST {{base_url}}/api/v1/observability/slis/{{sli_id}}/measurements
Authorization: Bearer {{admin_token}}
Content-Type: application/json
```
0.995
```
HTTP 204

# --- 3. GET SLIs (auth) — decode fence: bare Vec, self-seeded row present ---
GET {{base_url}}/api/v1/observability/slis
Authorization: Bearer {{access_token}}
HTTP 200
[Asserts]
jsonpath "$" isCollection
jsonpath "$[?(@.id=='{{sli_id}}')].name" contains "hurl-api-availability"
jsonpath "$[?(@.id=='{{sli_id}}')].metric_type" contains "Availability"

# --- 4. POST SLO (admin) → 201 SloDefinition; references sli_id ---
POST {{base_url}}/api/v1/observability/slos
Authorization: Bearer {{admin_token}}
Content-Type: application/json
```
{
  "name": "hurl-availability-slo",
  "sli_id": "{{sli_id}}",
  "target_value": 99.9,
  "error_budget": 0.1,
  "window_days": 30
}
```
HTTP 201
[Captures]
slo_id: jsonpath "$.id"
[Asserts]
jsonpath "$.id" isString
jsonpath "$.name" == "hurl-availability-slo"
jsonpath "$.sli_id" == "{{sli_id}}"
jsonpath "$.target_value" == 99.9
jsonpath "$.target_operator" exists
jsonpath "$.error_budget" isNumber
jsonpath "$.window_days" == 30
jsonpath "$.created_at" exists

# --- 5. GET SLOs (auth) — decode fence: bare Vec, self-seeded row present ---
GET {{base_url}}/api/v1/observability/slos
Authorization: Bearer {{access_token}}
HTTP 200
[Asserts]
jsonpath "$" isCollection
jsonpath "$[?(@.id=='{{slo_id}}')].name" contains "hurl-availability-slo"

# --- 6. POST evaluate SLO compliance (admin) → 200 bare Vec<SloCompliance> ---
POST {{base_url}}/api/v1/observability/slos/compliance/evaluate
Authorization: Bearer {{admin_token}}
HTTP 200
[Asserts]
jsonpath "$" isCollection

# --- 7. POST alert rule (admin) → 201 AlertRule; capture rule_id ---
POST {{base_url}}/api/v1/observability/alert-rules
Authorization: Bearer {{admin_token}}
Content-Type: application/json
```
{
  "name": "hurl-high-cpu",
  "metric": "cpu_usage",
  "condition": "gt",
  "threshold": 80.0,
  "severity": "Warning",
  "duration_sec": 0
}
```
HTTP 201
[Captures]
rule_id: jsonpath "$.id"
[Asserts]
jsonpath "$.id" isString
jsonpath "$.name" == "hurl-high-cpu"
jsonpath "$.metric" == "cpu_usage"
jsonpath "$.condition" == "gt"
jsonpath "$.threshold" == 80.0
jsonpath "$.severity" == "Warning"
jsonpath "$.duration_sec" == 0
jsonpath "$.enabled" isBoolean
jsonpath "$.created_at" exists

# --- 8. GET alert rules (auth) — decode fence: bare Vec, self-seeded row present ---
GET {{base_url}}/api/v1/observability/alert-rules
Authorization: Bearer {{access_token}}
HTTP 200
[Asserts]
jsonpath "$" isCollection
jsonpath "$[?(@.id=='{{rule_id}}')].name" contains "hurl-high-cpu"
jsonpath "$[?(@.id=='{{rule_id}}')].severity" contains "Warning"

# --- 9. POST evaluate alerts (admin) → 200 Vec<Alert>; metrics trip the rule ---
POST {{base_url}}/api/v1/observability/alerts/evaluate
Authorization: Bearer {{admin_token}}
Content-Type: application/json
```
{ "metrics": { "cpu_usage": 95.0 } }
```
HTTP 200
[Captures]
alert_id: jsonpath "$[0].id"
[Asserts]
jsonpath "$" isCollection
jsonpath "$[0].rule_id" exists
jsonpath "$[0].rule_name" exists
jsonpath "$[0].severity" exists
jsonpath "$[0].state" exists
jsonpath "$[0].message" isString
jsonpath "$[0].value" isNumber
jsonpath "$[0].fired_at" exists

# --- 10. POST resolve alert (admin) → 204 ---
POST {{base_url}}/api/v1/observability/alerts/{{alert_id}}/resolve
Authorization: Bearer {{admin_token}}
HTTP 204

# --- 11. DELETE alert rule (admin) → 204 ---
DELETE {{base_url}}/api/v1/observability/alert-rules/{{rule_id}}
Authorization: Bearer {{admin_token}}
HTTP 204

# --- 12. GET alert rules — deleted rule gone ---
GET {{base_url}}/api/v1/observability/alert-rules
Authorization: Bearer {{access_token}}
HTTP 200
[Asserts]
jsonpath "$" isCollection
jsonpath "$[?(@.id=='{{rule_id}}')]" not exists

# --- 13. RBAC: POST SLI as regular user → 403 (AdminUser) ---
POST {{base_url}}/api/v1/observability/slis
Authorization: Bearer {{access_token}}
Content-Type: application/json
```
{ "name": "forbidden", "metric_type": "Latency", "source": "x", "window_minutes": 5 }
```
HTTP 403

# --- 14. RBAC: POST measurement as regular user → 403 (AdminUser) ---
POST {{base_url}}/api/v1/observability/slis/{{sli_id}}/measurements
Authorization: Bearer {{access_token}}
Content-Type: application/json
```
0.5
```
HTTP 403

# --- 15. RBAC: POST alert rule as regular user → 403 (AdminUser) ---
POST {{base_url}}/api/v1/observability/alert-rules
Authorization: Bearer {{access_token}}
Content-Type: application/json
```
{ "name": "forbidden", "metric": "x", "condition": "gt", "threshold": 1.0, "severity": "Info", "duration_sec": 0 }
```
HTTP 403

# --- 16. No-auth POST SLI → 401 ---
POST {{base_url}}/api/v1/observability/slis
Content-Type: application/json
```
{ "name": "noauth", "metric_type": "Latency", "source": "x", "window_minutes": 5 }
```
HTTP 401

# --- 17. No-auth POST SLO → 401 ---
POST {{base_url}}/api/v1/observability/slos
Content-Type: application/json
```
{ "name": "noauth", "sli_id": "x", "target_value": 99.0, "error_budget": 0.1, "window_days": 7 }
```
HTTP 401

# --- 18. No-auth DELETE alert rule → 401 ---
DELETE {{base_url}}/api/v1/observability/alert-rules/{{rule_id}}
HTTP 401

# --- 20. No-auth POST evaluate alerts → 401 ---
POST {{base_url}}/api/v1/observability/alerts/evaluate
Content-Type: application/json
```
{ "metrics": { "x": 1.0 } }
```
HTTP 401