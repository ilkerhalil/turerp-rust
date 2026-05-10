//! Product Variants API endpoints (v1)

use actix_web::web;

pub mod categories;
pub mod products;
pub mod units;
pub mod variants;

/// Configure product variant routes for v1 API
pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::resource("/v1/products")
            .route(web::get().to(products::get_products))
            .route(web::post().to(products::create_product)),
    )
    .service(web::resource("/v1/products/search").route(web::get().to(products::search_products)))
    .service(
        web::resource("/v1/products/deleted").route(web::get().to(products::list_deleted_products)),
    )
    .service(
        web::resource("/v1/products/{id}")
            .route(web::get().to(products::get_product))
            .route(web::put().to(products::update_product))
            .route(web::delete().to(products::delete_product)),
    )
    .service(
        web::resource("/v1/products/{id}/restore").route(web::put().to(products::restore_product)),
    )
    .service(
        web::resource("/v1/products/{id}/destroy")
            .route(web::delete().to(products::destroy_product)),
    )
    .service(
        web::resource("/v1/categories")
            .route(web::get().to(categories::get_categories))
            .route(web::post().to(categories::create_category)),
    )
    .service(
        web::resource("/v1/categories/deleted")
            .route(web::get().to(categories::list_deleted_categories)),
    )
    .service(
        web::resource("/v1/categories/{id}")
            .route(web::get().to(categories::get_category))
            .route(web::put().to(categories::update_category))
            .route(web::delete().to(categories::delete_category)),
    )
    .service(
        web::resource("/v1/categories/{id}/restore")
            .route(web::put().to(categories::restore_category)),
    )
    .service(
        web::resource("/v1/categories/{id}/destroy")
            .route(web::delete().to(categories::destroy_category)),
    )
    .service(
        web::resource("/v1/units")
            .route(web::get().to(units::get_units))
            .route(web::post().to(units::create_unit)),
    )
    .service(web::resource("/v1/units/deleted").route(web::get().to(units::list_deleted_units)))
    .service(
        web::resource("/v1/units/{id}")
            .route(web::get().to(units::get_unit))
            .route(web::put().to(units::update_unit))
            .route(web::delete().to(units::delete_unit)),
    )
    .service(web::resource("/v1/units/{id}/restore").route(web::put().to(units::restore_unit)))
    .service(web::resource("/v1/units/{id}/destroy").route(web::delete().to(units::destroy_unit)))
    .service(
        web::resource("/v1/products/{product_id}/variants")
            .route(web::get().to(variants::get_variants_by_product))
            .route(web::post().to(variants::create_variant)),
    )
    .service(
        web::resource("/v1/products/{product_id}/variants/deleted")
            .route(web::get().to(variants::list_deleted_variants)),
    )
    .service(
        web::resource("/v1/variants/{id}")
            .route(web::get().to(variants::get_variant))
            .route(web::put().to(variants::update_variant))
            .route(web::delete().to(variants::delete_variant)),
    )
    .service(
        web::resource("/v1/variants/{id}/restore").route(web::put().to(variants::restore_variant)),
    )
    .service(
        web::resource("/v1/variants/{id}/destroy")
            .route(web::delete().to(variants::destroy_variant)),
    );
}
