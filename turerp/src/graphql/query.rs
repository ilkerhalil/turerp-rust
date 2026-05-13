//! GraphQL query root with tenant-isolated resolvers

use async_graphql::*;

use crate::common::pagination::PaginationParams;
use crate::graphql::context::GraphQlContext;
use crate::graphql::types::*;

/// Query root for the Turerp ERP GraphQL API
pub struct QueryRoot;

#[Object]
impl QueryRoot {
    /// Get a single user by ID
    async fn user(&self, ctx: &Context<'_>, id: ID) -> Result<Option<GraphQlUser>> {
        let gctx = ctx.data::<GraphQlContext>()?;
        let user_id: i64 = id.parse().map_err(|_| "Invalid user ID")?;

        let user = gctx
            .app_state
            .auth
            .user_service
            .get_user(user_id, gctx.tenant_id)
            .await
            .map_err(|e| e.to_string())?;

        Ok(Some(user.into()))
    }

    /// Get paginated list of users with optional search filter
    async fn users(
        &self,
        ctx: &Context<'_>,
        #[graphql(default = 1)] page: u32,
        #[graphql(default = 20)] per_page: u32,
        search: Option<String>,
    ) -> Result<UserConnection> {
        let gctx = ctx.data::<GraphQlContext>()?;
        let params = PaginationParams { page, per_page };
        params.validate()?;

        let result = if let Some(query) = search {
            let all = gctx
                .app_state
                .auth
                .user_service
                .get_all_users(gctx.tenant_id)
                .await
                .map_err(|e| e.to_string())?;
            let query_lower = query.to_lowercase();
            let filtered: Vec<_> = all
                .into_iter()
                .filter(|u| {
                    u.username.to_lowercase().contains(&query_lower)
                        || u.email.to_lowercase().contains(&query_lower)
                        || u.full_name.to_lowercase().contains(&query_lower)
                })
                .collect();
            let total = filtered.len() as u64;
            let items: Vec<_> = filtered
                .into_iter()
                .skip(params.offset() as usize)
                .take(params.limit() as usize)
                .collect();
            crate::common::pagination::PaginatedResult::new(items, page, per_page, total)
        } else {
            gctx.app_state
                .auth
                .user_service
                .get_all_users_paginated(gctx.tenant_id, page, per_page)
                .await
                .map_err(|e| e.to_string())?
        };

        let page_info = PageInfo::from_paginated(&result);
        Ok(UserConnection {
            items: result.items.into_iter().map(Into::into).collect(),
            page_info,
        })
    }

    /// Get a single employee by ID
    async fn employee(&self, ctx: &Context<'_>, id: ID) -> Result<Option<GraphQlEmployee>> {
        let gctx = ctx.data::<GraphQlContext>()?;
        let emp_id: i64 = id.parse().map_err(|_| "Invalid employee ID")?;

        let emp = gctx
            .app_state
            .hr
            .hr_service
            .get_employee(emp_id, gctx.tenant_id)
            .await
            .map_err(|e| e.to_string())?;

        Ok(Some(emp.into()))
    }

    /// Get paginated list of employees with optional filters
    async fn employees(
        &self,
        ctx: &Context<'_>,
        #[graphql(default = 1)] page: u32,
        #[graphql(default = 20)] per_page: u32,
        department: Option<String>,
        status: Option<GraphQlEmployeeStatus>,
    ) -> Result<EmployeeConnection> {
        let gctx = ctx.data::<GraphQlContext>()?;
        let params = PaginationParams { page, per_page };
        params.validate()?;

        let result = gctx
            .app_state
            .hr
            .hr_service
            .get_employees_paginated(gctx.tenant_id, page, per_page)
            .await
            .map_err(|e| e.to_string())?;

        let filtered: Vec<_> = result
            .items
            .into_iter()
            .filter(|e| {
                department
                    .as_ref()
                    .map(|d| {
                        e.department
                            .as_ref()
                            .map(|ed| ed.to_lowercase() == d.to_lowercase())
                            .unwrap_or(false)
                    })
                    .unwrap_or(true)
                    && status
                        .as_ref()
                        .map(|s| e.status == (*s).into())
                        .unwrap_or(true)
            })
            .collect();

        let total = filtered.len() as u64;
        let page_info = PageInfo {
            page,
            per_page,
            total,
            total_pages: if per_page == 0 {
                0
            } else {
                total.div_ceil(per_page as u64) as u32
            },
            has_next_page: page < (total.div_ceil(per_page as u64) as u32),
            has_previous_page: page > 1,
        };

        Ok(EmployeeConnection {
            items: filtered.into_iter().map(Into::into).collect(),
            page_info,
        })
    }

    /// Get a single invoice by ID
    async fn invoice(&self, ctx: &Context<'_>, id: ID) -> Result<Option<GraphQlInvoice>> {
        let gctx = ctx.data::<GraphQlContext>()?;
        let inv_id: i64 = id.parse().map_err(|_| "Invalid invoice ID")?;

        let inv = gctx
            .app_state
            .commerce
            .invoice_service
            .get_invoice(inv_id, gctx.tenant_id)
            .await
            .map_err(|e| e.to_string())?;

        Ok(Some(inv.into()))
    }

    /// Get paginated list of invoices with optional filters
    async fn invoices(
        &self,
        ctx: &Context<'_>,
        #[graphql(default = 1)] page: u32,
        #[graphql(default = 20)] per_page: u32,
        status: Option<GraphQlInvoiceStatus>,
        cari_id: Option<ID>,
    ) -> Result<InvoiceConnection> {
        let gctx = ctx.data::<GraphQlContext>()?;
        let params = PaginationParams { page, per_page };
        params.validate()?;

        let all = gctx
            .app_state
            .commerce
            .invoice_service
            .get_invoices_by_tenant(gctx.tenant_id)
            .await
            .map_err(|e| e.to_string())?;

        let filtered: Vec<_> = all
            .into_iter()
            .filter(|i| {
                let matches_status = status
                    .as_ref()
                    .map(|s| i.status == (*s).into())
                    .unwrap_or(true);
                let matches_cari = cari_id
                    .as_ref()
                    .map(|cid| cid.parse::<i64>().map(|c| i.cari_id == c).unwrap_or(false))
                    .unwrap_or(true);
                matches_status && matches_cari
            })
            .collect();

        let total = filtered.len() as u64;
        let items: Vec<_> = filtered
            .into_iter()
            .skip(params.offset() as usize)
            .take(params.limit() as usize)
            .collect();

        // Fetch full invoice responses with lines for each item
        let mut responses = Vec::with_capacity(items.len());
        for invoice in items {
            let full = gctx
                .app_state
                .commerce
                .invoice_service
                .get_invoice(invoice.id, gctx.tenant_id)
                .await
                .map_err(|e| e.to_string())?;
            responses.push(full);
        }

        let result =
            crate::common::pagination::PaginatedResult::new(responses, page, per_page, total);

        let page_info = PageInfo::from_paginated(&result);
        Ok(InvoiceConnection {
            items: result.items.into_iter().map(Into::into).collect(),
            page_info,
        })
    }

    /// Get a single product by ID
    async fn product(&self, ctx: &Context<'_>, id: ID) -> Result<Option<GraphQlProduct>> {
        let gctx = ctx.data::<GraphQlContext>()?;
        let prod_id: i64 = id.parse().map_err(|_| "Invalid product ID")?;

        let prod = gctx
            .app_state
            .commerce
            .product_service
            .get_product(prod_id, gctx.tenant_id)
            .await
            .map_err(|e| e.to_string())?;

        Ok(Some(prod.into()))
    }

    /// Get paginated list of products with optional search filter
    async fn products(
        &self,
        ctx: &Context<'_>,
        #[graphql(default = 1)] page: u32,
        #[graphql(default = 20)] per_page: u32,
        search: Option<String>,
        category_id: Option<ID>,
    ) -> Result<ProductConnection> {
        let gctx = ctx.data::<GraphQlContext>()?;
        let params = PaginationParams { page, per_page };
        params.validate()?;

        let result = gctx
            .app_state
            .commerce
            .product_service
            .get_products_paginated(gctx.tenant_id, page, per_page)
            .await
            .map_err(|e| e.to_string())?;

        let filtered: Vec<_> = result
            .items
            .into_iter()
            .filter(|p| {
                let matches_search = search
                    .as_ref()
                    .map(|q| {
                        let q_lower = q.to_lowercase();
                        p.name.to_lowercase().contains(&q_lower)
                            || p.code.to_lowercase().contains(&q_lower)
                    })
                    .unwrap_or(true);

                let matches_category = category_id
                    .as_ref()
                    .map(|cid| {
                        cid.parse::<i64>()
                            .map(|c| p.category_id == Some(c))
                            .unwrap_or(false)
                    })
                    .unwrap_or(true);

                matches_search && matches_category
            })
            .collect();

        let total = filtered.len() as u64;
        let page_info = PageInfo {
            page,
            per_page,
            total,
            total_pages: if per_page == 0 {
                0
            } else {
                total.div_ceil(per_page as u64) as u32
            },
            has_next_page: page < (total.div_ceil(per_page as u64) as u32),
            has_previous_page: page > 1,
        };

        Ok(ProductConnection {
            items: filtered.into_iter().map(|p| p.into()).collect(),
            page_info,
        })
    }

    /// Get a single cari by ID
    async fn cari(&self, ctx: &Context<'_>, id: ID) -> Result<Option<GraphQlCari>> {
        let gctx = ctx.data::<GraphQlContext>()?;
        let cari_id: i64 = id.parse().map_err(|_| "Invalid cari ID")?;

        let cari = gctx
            .app_state
            .commerce
            .cari_service
            .get_cari(cari_id, gctx.tenant_id)
            .await
            .map_err(|e| e.to_string())?;

        Ok(Some(cari.into()))
    }

    /// Get paginated list of cari accounts with optional filters
    async fn caris(
        &self,
        ctx: &Context<'_>,
        #[graphql(default = 1)] page: u32,
        #[graphql(default = 20)] per_page: u32,
        search: Option<String>,
        cari_type: Option<GraphQlCariType>,
    ) -> Result<CariConnection> {
        let gctx = ctx.data::<GraphQlContext>()?;
        let params = PaginationParams { page, per_page };
        params.validate()?;

        let result = gctx
            .app_state
            .commerce
            .cari_service
            .get_all_cari_paginated(gctx.tenant_id, page, per_page)
            .await
            .map_err(|e| e.to_string())?;

        let filtered: Vec<_> = result
            .items
            .into_iter()
            .filter(|c| {
                let matches_search = search
                    .as_ref()
                    .map(|q| {
                        let q_lower = q.to_lowercase();
                        c.name.to_lowercase().contains(&q_lower)
                            || c.code.to_lowercase().contains(&q_lower)
                    })
                    .unwrap_or(true);

                let matches_type = cari_type
                    .as_ref()
                    .map(|t| c.cari_type == (*t).into())
                    .unwrap_or(true);

                matches_search && matches_type
            })
            .collect();

        let total = filtered.len() as u64;
        let page_info = PageInfo {
            page,
            per_page,
            total,
            total_pages: if per_page == 0 {
                0
            } else {
                total.div_ceil(per_page as u64) as u32
            },
            has_next_page: page < (total.div_ceil(per_page as u64) as u32),
            has_previous_page: page > 1,
        };

        Ok(CariConnection {
            items: filtered.into_iter().map(Into::into).collect(),
            page_info,
        })
    }
}
