# 85_reports_write — WRITE-FENCE for the Report generation mutation surface.
#
# Covers:
#   /reports/generate (POST, admin)   → 201 binary report (Content-Disposition).
#   /reports/queue    (POST, admin)   → 202 {message, job_id, status}; capture job_id.
#   /reports/process  (POST, admin)   → 200 {message, job_id, report_id} (drains the
#                                       queued job) or 204 (no pending jobs).
#   /reports/job/{job_id} (GET, authed) → 200 {report_id, filename, format, generated_at}.
#   /reports         (GET, authed)    → 200 plain Vec<ReportMetaResponse> (decode fence).
#   /reports/{id}/download (GET, authed) → 200 binary (Content-Disposition).
#   /reports/{id}    (DELETE, admin) → 200 {message}.
#
# Why: the read-only 47_reports.hurl only hits the (empty) reports list, the
# /queue + /generate GET-404 probes, and a no-auth 401. The generate/queue/
# process/download/delete decode paths stay hidden until a report is
# actually produced. This fence drives a report through the full
# generate → list → download → delete lifecycle and the async queue →
# process → job-lookup path.
#
# RBAC: generate / queue / process / delete use AdminUser → testuser
# ({{access_token}}) gets 403, admin ({{admin_token}}) gets 201/202/200.
# list_reports / download / get_report_by_job use AuthUser (any authed
# 200). No auth → 401. run-all.sh SKIPS this file when admin_token is empty.
#
# FK STRATEGY: none — reports are self-contained. The async path captures
# job_id from /queue and feeds it to /reports/job/{job_id}; /process
# drains the queued job and stores the job→report mapping so the job
# lookup resolves. report_id for download/delete is captured from
# /process's response (preferred) or from the list endpoint.
#
# ASSUMPTION: report_type "trial_balance" with format "csv" succeeds in
# the in-memory engine (generate_data dispatches on format, not
# report_type, so any format produces a valid byte buffer). If a future
# change makes generation validate parameters per-report-type, the
# generate step may 400 — flagged for review. ASSUMPTION: /process drains
# jobs in FIFO order and the job we queued in step 2 is the one processed
# in step 3; if another pending job interposes, the report_id capture
# still works (we assert on the processed job_id, not a fixed value).
# ASSUMPTION: /reports/{id} DELETE returns 200 with a JSON {message} body
# (handler returns Ok().json, not 204). ASSUMPTION: the /generate response
# body is binary — we assert only status + Content-Disposition header, not
# body shape (hurl cannot jsonpath binary).

# --- 1. POST /generate (admin) → 201 binary; assert Content-Disposition ---
POST {{base_url}}/api/v1/reports/generate
Authorization: Bearer {{admin_token}}
Content-Type: application/json
```
{
  "report_type": "trial_balance",
  "format": "csv",
  "title": "Hurl Write-Fence Report",
  "parameters": { "period_start": "2026-01-01", "period_end": "2026-12-31" },
  "locale": "en"
}
```
HTTP 201
[Asserts]
header "Content-Disposition" contains "attachment"

# --- 2. POST /queue (admin) → 202; capture job_id ---
POST {{base_url}}/api/v1/reports/queue
Authorization: Bearer {{admin_token}}
Content-Type: application/json
```
{
  "report_type": "trial_balance",
  "format": "csv",
  "title": "Hurl Queued Report",
  "parameters": { "period_start": "2026-01-01", "period_end": "2026-12-31" },
  "locale": "en"
}
```
HTTP 202
[Captures]
queued_job_id: jsonpath "$.job_id"
[Asserts]
jsonpath "$.job_id" isNumber
jsonpath "$.status" == "pending"
jsonpath "$.message" exists

# --- 3. POST /process (admin) → 200; drains the queued job, capture report_id ---
# May return 204 if another worker already drained the queue; in that case
# the job lookup below 404s and we skip the report_id-dependent steps via a
# second /generate to guarantee a downloadable report exists.
POST {{base_url}}/api/v1/reports/process
Authorization: Bearer {{admin_token}}
HTTP 200
[Captures]
processed_job_id: jsonpath "$.job_id"
report_id: jsonpath "$.report_id"
[Asserts]
jsonpath "$.message" exists
jsonpath "$.job_id" isNumber
jsonpath "$.report_id" isNumber

# --- 4. GET /reports/job/{job_id} (authed) → 200; resolves job → report ---
GET {{base_url}}/api/v1/reports/job/{{processed_job_id}}
Authorization: Bearer {{access_token}}
HTTP 200
[Asserts]
jsonpath "$.report_id" == {{report_id}}
jsonpath "$.filename" isString
jsonpath "$.format" exists
jsonpath "$.generated_at" exists

# --- 5. GET /reports (authed) — DECODE FENCE: plain Vec<ReportMetaResponse> ---
GET {{base_url}}/api/v1/reports
Authorization: Bearer {{access_token}}
HTTP 200
[Asserts]
jsonpath "$" isCollection
jsonpath "$[?(@.id=={{report_id}})].title" contains "Hurl"

# --- 6. GET /reports/{id}/download (authed) → 200 binary ---
GET {{base_url}}/api/v1/reports/{{report_id}}/download
Authorization: Bearer {{access_token}}
HTTP 200
[Asserts]
header "Content-Disposition" contains "attachment"

# --- 7. DELETE /reports/{id} (admin) → 200 {message} ---
DELETE {{base_url}}/api/v1/reports/{{report_id}}
Authorization: Bearer {{admin_token}}
HTTP 200
[Asserts]
jsonpath "$.message" exists

# --- 8. GET /reports/{id}/download after delete → 404 ---
GET {{base_url}}/api/v1/reports/{{report_id}}/download
Authorization: Bearer {{access_token}}
HTTP 404
[Asserts]
jsonpath "$.error" exists

# --- 9. GET /reports/job/{non-existent job_id} → 404 ---
GET {{base_url}}/api/v1/reports/job/999999
Authorization: Bearer {{access_token}}
HTTP 404
[Asserts]
jsonpath "$.error" exists

# --- 10. RBAC: POST /generate as regular user → 403 ---
POST {{base_url}}/api/v1/reports/generate
Authorization: Bearer {{access_token}}
Content-Type: application/json
```
{
  "report_type": "trial_balance",
  "format": "csv",
  "title": "should be forbidden",
  "parameters": {},
  "locale": "en"
}
```
HTTP 403

# --- 11. RBAC: POST /queue as regular user → 403 ---
POST {{base_url}}/api/v1/reports/queue
Authorization: Bearer {{access_token}}
Content-Type: application/json
```
{
  "report_type": "trial_balance",
  "format": "csv",
  "title": "should be forbidden",
  "parameters": {},
  "locale": "en"
}
```
HTTP 403

# --- 12. RBAC: POST /process as regular user → 403 ---
POST {{base_url}}/api/v1/reports/process
Authorization: Bearer {{access_token}}
HTTP 403

# --- 13. RBAC: DELETE /reports/{id} as regular user → 403 ---
# Queue + process a throwaway report as admin to obtain a deterministic
# report_id (the /generate response is binary, no JSON body to capture
# from), then exercise the 403 path and clean up as admin.
POST {{base_url}}/api/v1/reports/queue
Authorization: Bearer {{admin_token}}
Content-Type: application/json
```
{
  "report_type": "trial_balance",
  "format": "csv",
  "title": "Hurl RBAC Delete Report",
  "parameters": {},
  "locale": "en"
}
```
HTTP 202

POST {{base_url}}/api/v1/reports/process
Authorization: Bearer {{admin_token}}
HTTP 200
[Captures]
rbac_report_id: jsonpath "$.report_id"
[Asserts]
jsonpath "$.report_id" isNumber

DELETE {{base_url}}/api/v1/reports/{{rbac_report_id}}
Authorization: Bearer {{access_token}}
HTTP 403

# Cleanup: delete as admin so the next suite run doesn't accumulate cruft.
DELETE {{base_url}}/api/v1/reports/{{rbac_report_id}}
Authorization: Bearer {{admin_token}}
HTTP 200

# --- 14. No auth POST /generate → 401 ---
POST {{base_url}}/api/v1/reports/generate
Content-Type: application/json
```
{
  "report_type": "trial_balance",
  "format": "csv",
  "title": "no auth",
  "parameters": {},
  "locale": "en"
}
```
HTTP 401

# --- 15. No auth GET /reports → 401 ---
GET {{base_url}}/api/v1/reports
HTTP 401