#!/usr/bin/env python3
"""Fix CRM postgres_repository.rs soft delete fields and filters."""

import re

with open('/home/ilker/projects/turerp-rust/turerp/src/domain/crm/postgres_repository.rs', 'r') as f:
    content = f.read()

# 1. Fix LeadRow - add deleted_at/deleted_by after total_count
content = content.replace(
    "    total_count: Option<i64>,\n}\n\nimpl From<LeadRow> for Lead {",
    "    total_count: Option<i64>,\n    deleted_at: Option<chrono::DateTime<chrono::Utc>>,\n    deleted_by: Option<i64>,\n}\n\nimpl From<LeadRow> for Lead {"
)

# 2. Fix LeadRow From impl
content = content.replace(
    "            deleted_at: None,\n            deleted_by: None,\n        }\n    }\n}\n\n/// PostgreSQL lead repository",
    "            deleted_at: row.deleted_at,\n            deleted_by: row.deleted_by,\n        }\n    }\n}\n\n/// PostgreSQL lead repository"
)

# 3. Fix OpportunityRow
content = content.replace(
    "    total_count: Option<i64>,\n}\n\nimpl From<OpportunityRow> for Opportunity {",
    "    total_count: Option<i64>,\n    deleted_at: Option<chrono::DateTime<chrono::Utc>>,\n    deleted_by: Option<i64>,\n}\n\nimpl From<OpportunityRow> for Opportunity {"
)

content = content.replace(
    "            deleted_at: None,\n            deleted_by: None,\n        }\n    }\n}\n\n/// PostgreSQL opportunity repository",
    "            deleted_at: row.deleted_at,\n            deleted_by: row.deleted_by,\n        }\n    }\n}\n\n/// PostgreSQL opportunity repository"
)

# 4. Fix CampaignRow
content = content.replace(
    "    total_count: Option<i64>,\n}\n\nimpl From<CampaignRow> for Campaign {",
    "    total_count: Option<i64>,\n    deleted_at: Option<chrono::DateTime<chrono::Utc>>,\n    deleted_by: Option<i64>,\n}\n\nimpl From<CampaignRow> for Campaign {"
)

content = content.replace(
    "            deleted_at: None,\n            deleted_by: None,\n        }\n    }\n}\n\n/// PostgreSQL campaign repository",
    "            deleted_at: row.deleted_at,\n            deleted_by: row.deleted_by,\n        }\n    }\n}\n\n/// PostgreSQL campaign repository"
)

# 5. Fix TicketRow
content = content.replace(
    "    total_count: Option<i64>,\n}\n\nimpl From<TicketRow> for Ticket {",
    "    total_count: Option<i64>,\n    deleted_at: Option<chrono::DateTime<chrono::Utc>>,\n    deleted_by: Option<i64>,\n}\n\nimpl From<TicketRow> for Ticket {"
)

content = content.replace(
    "            deleted_at: None,\n            deleted_by: None,\n        }\n    }\n}\n\n/// PostgreSQL ticket repository",
    "            deleted_at: row.deleted_at,\n            deleted_by: row.deleted_by,\n        }\n    }\n}\n\n/// PostgreSQL ticket repository"
)

# 6. Add deleted_at, deleted_by to all Lead SELECT/RETURNING clauses
# For RETURNING/SELECT with created_at, updated_at - add deleted_at, deleted_by after updated_at
lead_patterns = [
    ("RETURNING id, tenant_id, name, company, email, phone, source,\n                      status, assigned_to, converted_to_customer_id, notes, created_at, updated_at",
     "RETURNING id, tenant_id, name, company, email, phone, source,\n                      status, assigned_to, converted_to_customer_id, notes, created_at, updated_at, deleted_at, deleted_by"),
    ("SELECT id, tenant_id, name, company, email, phone, source,\n                   status, assigned_to, converted_to_customer_id, notes, created_at, updated_at",
     "SELECT id, tenant_id, name, company, email, phone, source,\n                   status, assigned_to, converted_to_customer_id, notes, created_at, updated_at, deleted_at, deleted_by"),
    ("SELECT id, tenant_id, name, company, email, phone, source,\n                   status, assigned_to, converted_to_customer_id, notes, created_at, updated_at,\n                   COUNT(*) OVER() as total_count",
     "SELECT id, tenant_id, name, company, email, phone, source,\n                   status, assigned_to, converted_to_customer_id, notes, created_at, updated_at, deleted_at, deleted_by,\n                   COUNT(*) OVER() as total_count"),
    ("RETURNING id, tenant_id, name, company, email, phone, source,\n                      status, assigned_to, converted_to_customer_id, notes, created_at, updated_at",
     "RETURNING id, tenant_id, name, company, email, phone, source,\n                      status, assigned_to, converted_to_customer_id, notes, created_at, updated_at, deleted_at, deleted_by"),
]

for old, new in lead_patterns:
    content = content.replace(old, new)

# 7. Add deleted_at, deleted_by to Opportunity SELECT/RETURNING
opp_patterns = [
    ("RETURNING id, tenant_id, lead_id, name, customer_id, value, probability,\n                      expected_close_date, status, assigned_to, notes, created_at, updated_at",
     "RETURNING id, tenant_id, lead_id, name, customer_id, value, probability,\n                      expected_close_date, status, assigned_to, notes, created_at, updated_at, deleted_at, deleted_by"),
    ("SELECT id, tenant_id, lead_id, name, customer_id, value, probability,\n                   expected_close_date, status, assigned_to, notes, created_at, updated_at",
     "SELECT id, tenant_id, lead_id, name, customer_id, value, probability,\n                   expected_close_date, status, assigned_to, notes, created_at, updated_at, deleted_at, deleted_by"),
    ("SELECT id, tenant_id, lead_id, name, customer_id, value, probability,\n                   expected_close_date, status, assigned_to, notes, created_at, updated_at,\n                   COUNT(*) OVER() as total_count",
     "SELECT id, tenant_id, lead_id, name, customer_id, value, probability,\n                   expected_close_date, status, assigned_to, notes, created_at, updated_at, deleted_at, deleted_by,\n                   COUNT(*) OVER() as total_count"),
]

for old, new in opp_patterns:
    content = content.replace(old, new)

# 8. Add deleted_at, deleted_by to Campaign SELECT/RETURNING
camp_patterns = [
    ("RETURNING id, tenant_id, name, description, campaign_type, status,\n                      budget, actual_cost, start_date, end_date, created_at, updated_at",
     "RETURNING id, tenant_id, name, description, campaign_type, status,\n                      budget, actual_cost, start_date, end_date, created_at, updated_at, deleted_at, deleted_by"),
    ("SELECT id, tenant_id, name, description, campaign_type, status,\n                   budget, actual_cost, start_date, end_date, created_at, updated_at",
     "SELECT id, tenant_id, name, description, campaign_type, status,\n                   budget, actual_cost, start_date, end_date, created_at, updated_at, deleted_at, deleted_by"),
    ("SELECT id, tenant_id, name, description, campaign_type, status,\n                   budget, actual_cost, start_date, end_date, created_at, updated_at,\n                   COUNT(*) OVER() as total_count",
     "SELECT id, tenant_id, name, description, campaign_type, status,\n                   budget, actual_cost, start_date, end_date, created_at, updated_at, deleted_at, deleted_by,\n                   COUNT(*) OVER() as total_count"),
]

for old, new in camp_patterns:
    content = content.replace(old, new)

# 9. Add deleted_at, deleted_by to Ticket SELECT/RETURNING
ticket_patterns = [
    ("RETURNING id, tenant_id, ticket_number, subject, description,\n                      customer_id, assigned_to, status, priority, category,\n                      resolved_at, created_at, updated_at",
     "RETURNING id, tenant_id, ticket_number, subject, description,\n                      customer_id, assigned_to, status, priority, category,\n                      resolved_at, created_at, updated_at, deleted_at, deleted_by"),
    ("SELECT id, tenant_id, ticket_number, subject, description,\n                   customer_id, assigned_to, status, priority, category,\n                   resolved_at, created_at, updated_at",
     "SELECT id, tenant_id, ticket_number, subject, description,\n                   customer_id, assigned_to, status, priority, category,\n                   resolved_at, created_at, updated_at, deleted_at, deleted_by"),
    ("SELECT id, tenant_id, ticket_number, subject, description,\n                   customer_id, assigned_to, status, priority, category,\n                   resolved_at, created_at, updated_at, COUNT(*) OVER() as total_count",
     "SELECT id, tenant_id, ticket_number, subject, description,\n                   customer_id, assigned_to, status, priority, category,\n                   resolved_at, created_at, updated_at, deleted_at, deleted_by, COUNT(*) OVER() as total_count"),
]

for old, new in ticket_patterns:
    content = content.replace(old, new)

# 10. Add AND deleted_at IS NULL to Lead find queries
content = content.replace(
    "FROM leads\n            WHERE id = $1 AND tenant_id = $2",
    "FROM leads\n            WHERE id = $1 AND tenant_id = $2 AND deleted_at IS NULL"
)
content = content.replace(
    "FROM leads\n            WHERE tenant_id = $1\n            ORDER BY created_at DESC",
    "FROM leads\n            WHERE tenant_id = $1 AND deleted_at IS NULL\n            ORDER BY created_at DESC"
)
content = content.replace(
    "FROM leads\n            WHERE tenant_id = $1 AND status = $2\n            ORDER BY created_at DESC",
    "FROM leads\n            WHERE tenant_id = $1 AND status = $2 AND deleted_at IS NULL\n            ORDER BY created_at DESC"
)
content = content.replace(
    "FROM leads\n            WHERE tenant_id = $1\n            ORDER BY id DESC",
    "FROM leads\n            WHERE tenant_id = $1 AND deleted_at IS NULL\n            ORDER BY id DESC"
)
content = content.replace(
    "FROM leads\n            WHERE tenant_id = $1 AND status = $2\n            ORDER BY id DESC",
    "FROM leads\n            WHERE tenant_id = $1 AND status = $2 AND deleted_at IS NULL\n            ORDER BY id DESC"
)
content = content.replace(
    "UPDATE leads\n            SET status = $1, updated_at = NOW()\n            WHERE id = $2",
    "UPDATE leads\n            SET status = $1, updated_at = NOW()\n            WHERE id = $2 AND deleted_at IS NULL"
)
content = content.replace(
    "UPDATE leads\n            SET status = 'Converted', converted_to_customer_id = $1, updated_at = NOW()\n            WHERE id = $2",
    "UPDATE leads\n            SET status = 'Converted', converted_to_customer_id = $1, updated_at = NOW()\n            WHERE id = $2 AND deleted_at IS NULL"
)

# 11. Add AND deleted_at IS NULL to Opportunity find queries
content = content.replace(
    "FROM opportunities\n            WHERE id = $1 AND tenant_id = $2",
    "FROM opportunities\n            WHERE id = $1 AND tenant_id = $2 AND deleted_at IS NULL"
)
content = content.replace(
    "FROM opportunities\n            WHERE tenant_id = $1\n            ORDER BY created_at DESC",
    "FROM opportunities\n            WHERE tenant_id = $1 AND deleted_at IS NULL\n            ORDER BY created_at DESC"
)
content = content.replace(
    "FROM opportunities\n            WHERE tenant_id = $1 AND status = $2\n            ORDER BY created_at DESC",
    "FROM opportunities\n            WHERE tenant_id = $1 AND status = $2 AND deleted_at IS NULL\n            ORDER BY created_at DESC"
)
content = content.replace(
    "FROM opportunities\n            WHERE tenant_id = $1\n            ORDER BY id DESC",
    "FROM opportunities\n            WHERE tenant_id = $1 AND deleted_at IS NULL\n            ORDER BY id DESC"
)
content = content.replace(
    "FROM opportunities\n            WHERE tenant_id = $1 AND status = $2\n            ORDER BY id DESC",
    "FROM opportunities\n            WHERE tenant_id = $1 AND status = $2 AND deleted_at IS NULL\n            ORDER BY id DESC"
)
content = content.replace(
    "FROM opportunities\n            WHERE customer_id = $1\n            ORDER BY created_at DESC",
    "FROM opportunities\n            WHERE customer_id = $1 AND deleted_at IS NULL\n            ORDER BY created_at DESC"
)
content = content.replace(
    "UPDATE opportunities\n            SET status = $1, updated_at = NOW()\n            WHERE id = $2",
    "UPDATE opportunities\n            SET status = $1, updated_at = NOW()\n            WHERE id = $2 AND deleted_at IS NULL"
)

# 12. Add AND deleted_at IS NULL to Campaign find queries
content = content.replace(
    "FROM campaigns\n            WHERE id = $1 AND tenant_id = $2",
    "FROM campaigns\n            WHERE id = $1 AND tenant_id = $2 AND deleted_at IS NULL"
)
content = content.replace(
    "FROM campaigns\n            WHERE tenant_id = $1\n            ORDER BY created_at DESC",
    "FROM campaigns\n            WHERE tenant_id = $1 AND deleted_at IS NULL\n            ORDER BY created_at DESC"
)
content = content.replace(
    "FROM campaigns\n            WHERE tenant_id = $1 AND status = $2\n            ORDER BY created_at DESC",
    "FROM campaigns\n            WHERE tenant_id = $1 AND status = $2 AND deleted_at IS NULL\n            ORDER BY created_at DESC"
)
content = content.replace(
    "FROM campaigns\n            WHERE tenant_id = $1\n            ORDER BY id DESC",
    "FROM campaigns\n            WHERE tenant_id = $1 AND deleted_at IS NULL\n            ORDER BY id DESC"
)
content = content.replace(
    "FROM campaigns\n            WHERE tenant_id = $1 AND status = $2\n            ORDER BY id DESC",
    "FROM campaigns\n            WHERE tenant_id = $1 AND status = $2 AND deleted_at IS NULL\n            ORDER BY id DESC"
)
content = content.replace(
    "UPDATE campaigns\n            SET status = $1, updated_at = NOW()\n            WHERE id = $2",
    "UPDATE campaigns\n            SET status = $1, updated_at = NOW()\n            WHERE id = $2 AND deleted_at IS NULL"
)

# 13. Add AND deleted_at IS NULL to Ticket find queries
content = content.replace(
    "FROM tickets\n            WHERE id = $1 AND tenant_id = $2",
    "FROM tickets\n            WHERE id = $1 AND tenant_id = $2 AND deleted_at IS NULL"
)
content = content.replace(
    "FROM tickets\n            WHERE tenant_id = $1\n            ORDER BY created_at DESC",
    "FROM tickets\n            WHERE tenant_id = $1 AND deleted_at IS NULL\n            ORDER BY created_at DESC"
)
content = content.replace(
    "FROM tickets\n            WHERE tenant_id = $1 AND status = $2\n            ORDER BY created_at DESC",
    "FROM tickets\n            WHERE tenant_id = $1 AND status = $2 AND deleted_at IS NULL\n            ORDER BY created_at DESC"
)
content = content.replace(
    "FROM tickets\n            WHERE tenant_id = $1\n            ORDER BY id DESC",
    "FROM tickets\n            WHERE tenant_id = $1 AND deleted_at IS NULL\n            ORDER BY id DESC"
)
content = content.replace(
    "FROM tickets\n            WHERE tenant_id = $1 AND status = $2\n            ORDER BY id DESC",
    "FROM tickets\n            WHERE tenant_id = $1 AND status = $2 AND deleted_at IS NULL\n            ORDER BY id DESC"
)
content = content.replace(
    "FROM tickets\n            WHERE tenant_id = $1 AND ticket_number = $2",
    "FROM tickets\n            WHERE tenant_id = $1 AND ticket_number = $2 AND deleted_at IS NULL"
)
content = content.replace(
    "FROM tickets\n            WHERE assigned_to = $1\n            ORDER BY created_at DESC",
    "FROM tickets\n            WHERE assigned_to = $1 AND deleted_at IS NULL\n            ORDER BY created_at DESC"
)
content = content.replace(
    "UPDATE tickets\n            SET status = $1, updated_at = NOW()\n            WHERE id = $2",
    "UPDATE tickets\n            SET status = $1, updated_at = NOW()\n            WHERE id = $2 AND deleted_at IS NULL"
)
content = content.replace(
    "UPDATE tickets\n            SET status = 'Resolved', resolved_at = NOW(), updated_at = NOW()\n            WHERE id = $1",
    "UPDATE tickets\n            SET status = 'Resolved', resolved_at = NOW(), updated_at = NOW()\n            WHERE id = $1 AND deleted_at IS NULL"
)

with open('/home/ilker/projects/turerp-rust/turerp/src/domain/crm/postgres_repository.rs', 'w') as f:
    f.write(content)

print("CRM postgres_repository.rs updated")
