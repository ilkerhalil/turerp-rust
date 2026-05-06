#!/usr/bin/env python3
"""Fix manufacturing postgres_repository.rs soft delete fields and filters."""

import re

with open('/home/ilker/projects/turerp-rust/turerp/src/domain/manufacturing/postgres_repository.rs', 'r') as f:
    content = f.read()

# 1. Fix WorkOrderRow - add deleted_at/deleted_by after total_count
content = content.replace(
    "    total_count: Option<i64>,\n}\n\nimpl From<WorkOrderRow> for WorkOrder {",
    "    total_count: Option<i64>,\n    deleted_at: Option<chrono::DateTime<chrono::Utc>>,\n    deleted_by: Option<i64>,\n}\n\nimpl From<WorkOrderRow> for WorkOrder {"
)

# 2. Fix WorkOrderRow From impl
content = content.replace(
    "            deleted_at: None,\n            deleted_by: None,\n        }\n    }\n}\n\n/// Database row representation for WorkOrderOperation",
    "            deleted_at: row.deleted_at,\n            deleted_by: row.deleted_by,\n        }\n    }\n}\n\n/// Database row representation for WorkOrderOperation"
)

# 3. Fix BillOfMaterialsRow
content = content.replace(
    "    updated_at: chrono::DateTime<chrono::Utc>,\n}\n\nimpl From<BillOfMaterialsRow> for BillOfMaterials {",
    "    updated_at: chrono::DateTime<chrono::Utc>,\n    deleted_at: Option<chrono::DateTime<chrono::Utc>>,\n    deleted_by: Option<i64>,\n}\n\nimpl From<BillOfMaterialsRow> for BillOfMaterials {"
)

content = content.replace(
    "            deleted_at: None,\n            deleted_by: None,\n        }\n    }\n}\n\n/// Database row representation for BillOfMaterialsLine",
    "            deleted_at: row.deleted_at,\n            deleted_by: row.deleted_by,\n        }\n    }\n}\n\n/// Database row representation for BillOfMaterialsLine"
)

# 4. Fix RoutingRow
content = content.replace(
    "    updated_at: chrono::DateTime<chrono::Utc>,\n}\n\nimpl From<RoutingRow> for Routing {",
    "    updated_at: chrono::DateTime<chrono::Utc>,\n    deleted_at: Option<chrono::DateTime<chrono::Utc>>,\n    deleted_by: Option<i64>,\n}\n\nimpl From<RoutingRow> for Routing {"
)

content = content.replace(
    "            deleted_at: None,\n            deleted_by: None,\n        }\n    }\n}\n\n/// Database row representation for RoutingOperation",
    "            deleted_at: row.deleted_at,\n            deleted_by: row.deleted_by,\n        }\n    }\n}\n\n/// Database row representation for RoutingOperation"
)

# 5. WorkOrder SELECT/RETURNING patterns
wo_patterns = [
    # create RETURNING
    ("RETURNING id, tenant_id, name, product_id, quantity, bom_id, routing_id,\n                      status, priority, planned_start, planned_end,\n                      actual_start, actual_end, created_at, updated_at",
     "RETURNING id, tenant_id, name, product_id, quantity, bom_id, routing_id,\n                      status, priority, planned_start, planned_end,\n                      actual_start, actual_end, created_at, updated_at, deleted_at, deleted_by"),
    # find_by_id SELECT
    ("SELECT id, tenant_id, name, product_id, quantity, bom_id, routing_id,\n                   status, priority, planned_start, planned_end,\n                   actual_start, actual_end, created_at, updated_at\n            FROM work_orders\n            WHERE id = $1 AND tenant_id = $2 AND deleted_at IS NULL",
     "SELECT id, tenant_id, name, product_id, quantity, bom_id, routing_id,\n                   status, priority, planned_start, planned_end,\n                   actual_start, actual_end, created_at, updated_at, deleted_at, deleted_by\n            FROM work_orders\n            WHERE id = $1 AND tenant_id = $2 AND deleted_at IS NULL"),
    # find_by_tenant SELECT (missing deleted_at IS NULL)
    ("SELECT id, tenant_id, name, product_id, quantity, bom_id, routing_id,\n                   status, priority, planned_start, planned_end,\n                   actual_start, actual_end, created_at, updated_at\n            FROM work_orders\n            WHERE tenant_id = $1\n            ORDER BY created_at DESC",
     "SELECT id, tenant_id, name, product_id, quantity, bom_id, routing_id,\n                   status, priority, planned_start, planned_end,\n                   actual_start, actual_end, created_at, updated_at, deleted_at, deleted_by\n            FROM work_orders\n            WHERE tenant_id = $1 AND deleted_at IS NULL\n            ORDER BY created_at DESC"),
    # find_by_tenant_paginated SELECT (missing deleted_at IS NULL)
    ("SELECT id, tenant_id, name, product_id, quantity, bom_id, routing_id,\n                   status, priority, planned_start, planned_end,\n                   actual_start, actual_end, created_at, updated_at,\n                   COUNT(*) OVER() as total_count\n            FROM work_orders\n            WHERE tenant_id = $1\n            ORDER BY id DESC",
     "SELECT id, tenant_id, name, product_id, quantity, bom_id, routing_id,\n                   status, priority, planned_start, planned_end,\n                   actual_start, actual_end, created_at, updated_at, deleted_at, deleted_by,\n                   COUNT(*) OVER() as total_count\n            FROM work_orders\n            WHERE tenant_id = $1 AND deleted_at IS NULL\n            ORDER BY id DESC"),
    # find_by_product SELECT (missing deleted_at IS NULL)
    ("SELECT id, tenant_id, name, product_id, quantity, bom_id, routing_id,\n                   status, priority, planned_start, planned_end,\n                   actual_start, actual_end, created_at, updated_at\n            FROM work_orders\n            WHERE product_id = $1\n            ORDER BY created_at DESC",
     "SELECT id, tenant_id, name, product_id, quantity, bom_id, routing_id,\n                   status, priority, planned_start, planned_end,\n                   actual_start, actual_end, created_at, updated_at, deleted_at, deleted_by\n            FROM work_orders\n            WHERE product_id = $1 AND deleted_at IS NULL\n            ORDER BY created_at DESC"),
    # find_by_status SELECT (missing deleted_at IS NULL)
    ("SELECT id, tenant_id, name, product_id, quantity, bom_id, routing_id,\n                   status, priority, planned_start, planned_end,\n                   actual_start, actual_end, created_at, updated_at\n            FROM work_orders\n            WHERE tenant_id = $1 AND status = $2\n            ORDER BY created_at DESC",
     "SELECT id, tenant_id, name, product_id, quantity, bom_id, routing_id,\n                   status, priority, planned_start, planned_end,\n                   actual_start, actual_end, created_at, updated_at, deleted_at, deleted_by\n            FROM work_orders\n            WHERE tenant_id = $1 AND status = $2 AND deleted_at IS NULL\n            ORDER BY created_at DESC"),
    # update_status RETURNING + WHERE (missing deleted_at IS NULL)
    ("UPDATE work_orders\n            SET status = $1, updated_at = NOW()\n            WHERE id = $2\n            RETURNING id, tenant_id, name, product_id, quantity, bom_id, routing_id,\n                      status, priority, planned_start, planned_end,\n                      actual_start, actual_end, created_at, updated_at",
     "UPDATE work_orders\n            SET status = $1, updated_at = NOW()\n            WHERE id = $2 AND deleted_at IS NULL\n            RETURNING id, tenant_id, name, product_id, quantity, bom_id, routing_id,\n                      status, priority, planned_start, planned_end,\n                      actual_start, actual_end, created_at, updated_at, deleted_at, deleted_by"),
    # restore RETURNING
    ("RETURNING id, tenant_id, name, product_id, quantity, bom_id, routing_id,\n                      status, priority, planned_start, planned_end,\n                      actual_start, actual_end, created_at, updated_at\n            \"#,\n        )\n        .bind(id)\n        .bind(tenant_id)\n        .fetch_one(&*self.pool)\n        .await\n        .map_err(|e| map_sqlx_error(e, \"WorkOrder\"))?;\n\n        Ok(row.into())\n    }\n\n    async fn find_deleted",
     "RETURNING id, tenant_id, name, product_id, quantity, bom_id, routing_id,\n                      status, priority, planned_start, planned_end,\n                      actual_start, actual_end, created_at, updated_at, deleted_at, deleted_by\n            \"#,\n        )\n        .bind(id)\n        .bind(tenant_id)\n        .fetch_one(&*self.pool)\n        .await\n        .map_err(|e| map_sqlx_error(e, \"WorkOrder\"))?;\n\n        Ok(row.into())\n    }\n\n    async fn find_deleted"),
    # find_deleted SELECT
    ("SELECT id, tenant_id, name, product_id, quantity, bom_id, routing_id,\n                   status, priority, planned_start, planned_end,\n                   actual_start, actual_end, created_at, updated_at\n            FROM work_orders\n            WHERE tenant_id = $1 AND deleted_at IS NOT NULL",
     "SELECT id, tenant_id, name, product_id, quantity, bom_id, routing_id,\n                   status, priority, planned_start, planned_end,\n                   actual_start, actual_end, created_at, updated_at, deleted_at, deleted_by\n            FROM work_orders\n            WHERE tenant_id = $1 AND deleted_at IS NOT NULL"),
]

for old, new in wo_patterns:
    content = content.replace(old, new)

# 6. BillOfMaterials SELECT/RETURNING patterns
bom_patterns = [
    # create RETURNING
    ("RETURNING id, tenant_id, product_id, version, is_active, is_primary,\n                      valid_from, valid_to, created_at, updated_at",
     "RETURNING id, tenant_id, product_id, version, is_active, is_primary,\n                      valid_from, valid_to, created_at, updated_at, deleted_at, deleted_by"),
    # find_by_id SELECT
    ("SELECT id, tenant_id, product_id, version, is_active, is_primary,\n                   valid_from, valid_to, created_at, updated_at\n            FROM bills_of_materials\n            WHERE id = $1 AND tenant_id = $2 AND deleted_at IS NULL",
     "SELECT id, tenant_id, product_id, version, is_active, is_primary,\n                   valid_from, valid_to, created_at, updated_at, deleted_at, deleted_by\n            FROM bills_of_materials\n            WHERE id = $1 AND tenant_id = $2 AND deleted_at IS NULL"),
    # find_by_product SELECT (missing deleted_at IS NULL)
    ("SELECT id, tenant_id, product_id, version, is_active, is_primary,\n                   valid_from, valid_to, created_at, updated_at\n            FROM bills_of_materials\n            WHERE product_id = $1\n            ORDER BY created_at DESC",
     "SELECT id, tenant_id, product_id, version, is_active, is_primary,\n                   valid_from, valid_to, created_at, updated_at, deleted_at, deleted_by\n            FROM bills_of_materials\n            WHERE product_id = $1 AND deleted_at IS NULL\n            ORDER BY created_at DESC"),
    # find_primary_by_product SELECT (missing deleted_at IS NULL)
    ("SELECT id, tenant_id, product_id, version, is_active, is_primary,\n                   valid_from, valid_to, created_at, updated_at\n            FROM bills_of_materials\n            WHERE product_id = $1 AND is_primary = true AND is_active = true",
     "SELECT id, tenant_id, product_id, version, is_active, is_primary,\n                   valid_from, valid_to, created_at, updated_at, deleted_at, deleted_by\n            FROM bills_of_materials\n            WHERE product_id = $1 AND is_primary = true AND is_active = true AND deleted_at IS NULL"),
    # restore RETURNING
    ("RETURNING id, tenant_id, product_id, version, is_active, is_primary,\n                      valid_from, valid_to, created_at, updated_at\n            \"#,\n        )\n        .bind(id)\n        .bind(tenant_id)\n        .fetch_one(&*self.pool)\n        .await\n        .map_err(|e| map_sqlx_error(e, \"BillOfMaterials\"))?;\n\n        Ok(row.into())\n    }\n\n    async fn find_deleted",
     "RETURNING id, tenant_id, product_id, version, is_active, is_primary,\n                      valid_from, valid_to, created_at, updated_at, deleted_at, deleted_by\n            \"#,\n        )\n        .bind(id)\n        .bind(tenant_id)\n        .fetch_one(&*self.pool)\n        .await\n        .map_err(|e| map_sqlx_error(e, \"BillOfMaterials\"))?;\n\n        Ok(row.into())\n    }\n\n    async fn find_deleted"),
    # find_deleted SELECT
    ("SELECT id, tenant_id, product_id, version, is_active, is_primary,\n                   valid_from, valid_to, created_at, updated_at\n            FROM bills_of_materials\n            WHERE tenant_id = $1 AND deleted_at IS NOT NULL",
     "SELECT id, tenant_id, product_id, version, is_active, is_primary,\n                   valid_from, valid_to, created_at, updated_at, deleted_at, deleted_by\n            FROM bills_of_materials\n            WHERE tenant_id = $1 AND deleted_at IS NOT NULL"),
]

for old, new in bom_patterns:
    content = content.replace(old, new)

# 7. Routing SELECT/RETURNING patterns
routing_patterns = [
    # create RETURNING
    ("RETURNING id, tenant_id, product_id, version, is_active, is_primary,\n                      created_at, updated_at",
     "RETURNING id, tenant_id, product_id, version, is_active, is_primary,\n                      created_at, updated_at, deleted_at, deleted_by"),
    # find_by_id SELECT
    ("SELECT id, tenant_id, product_id, version, is_active, is_primary,\n                   created_at, updated_at\n            FROM routings\n            WHERE id = $1 AND tenant_id = $2 AND deleted_at IS NULL",
     "SELECT id, tenant_id, product_id, version, is_active, is_primary,\n                   created_at, updated_at, deleted_at, deleted_by\n            FROM routings\n            WHERE id = $1 AND tenant_id = $2 AND deleted_at IS NULL"),
    # find_by_product SELECT (missing deleted_at IS NULL)
    ("SELECT id, tenant_id, product_id, version, is_active, is_primary,\n                   created_at, updated_at\n            FROM routings\n            WHERE product_id = $1\n            ORDER BY created_at DESC",
     "SELECT id, tenant_id, product_id, version, is_active, is_primary,\n                   created_at, updated_at, deleted_at, deleted_by\n            FROM routings\n            WHERE product_id = $1 AND deleted_at IS NULL\n            ORDER BY created_at DESC"),
    # find_primary_by_product SELECT (missing deleted_at IS NULL)
    ("SELECT id, tenant_id, product_id, version, is_active, is_primary,\n                   created_at, updated_at\n            FROM routings\n            WHERE product_id = $1 AND is_primary = true AND is_active = true",
     "SELECT id, tenant_id, product_id, version, is_active, is_primary,\n                   created_at, updated_at, deleted_at, deleted_by\n            FROM routings\n            WHERE product_id = $1 AND is_primary = true AND is_active = true AND deleted_at IS NULL"),
    # restore RETURNING
    ("RETURNING id, tenant_id, product_id, version, is_active, is_primary,\n                      created_at, updated_at\n            \"#,\n        )\n        .bind(id)\n        .bind(tenant_id)\n        .fetch_one(&*self.pool)\n        .await\n        .map_err(|e| map_sqlx_error(e, \"Routing\"))?;\n\n        Ok(row.into())\n    }\n\n    async fn find_deleted",
     "RETURNING id, tenant_id, product_id, version, is_active, is_primary,\n                      created_at, updated_at, deleted_at, deleted_by\n            \"#,\n        )\n        .bind(id)\n        .bind(tenant_id)\n        .fetch_one(&*self.pool)\n        .await\n        .map_err(|e| map_sqlx_error(e, \"Routing\"))?;\n\n        Ok(row.into())\n    }\n\n    async fn find_deleted"),
    # find_deleted SELECT
    ("SELECT id, tenant_id, product_id, version, is_active, is_primary,\n                   created_at, updated_at\n            FROM routings\n            WHERE tenant_id = $1 AND deleted_at IS NOT NULL",
     "SELECT id, tenant_id, product_id, version, is_active, is_primary,\n                   created_at, updated_at, deleted_at, deleted_by\n            FROM routings\n            WHERE tenant_id = $1 AND deleted_at IS NOT NULL"),
]

for old, new in routing_patterns:
    content = content.replace(old, new)

with open('/home/ilker/projects/turerp-rust/turerp/src/domain/manufacturing/postgres_repository.rs', 'w') as f:
    f.write(content)

print("Manufacturing postgres_repository.rs updated")
