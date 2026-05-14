//! Barcode service for business logic

use barcodes::common::traits::BarcodeEncoder;

use crate::common::pagination::PaginatedResult;
use crate::domain::barcode::model::{
    BarcodeConfig, BarcodeType, CreateBarcode, GenerateBarcodeRequest,
};
use crate::domain::barcode::repository::BoxBarcodeRepository;
use crate::error::ApiError;

/// Barcode generation service
#[derive(Clone)]
pub struct BarcodeService {
    repo: BoxBarcodeRepository,
}

impl BarcodeService {
    pub fn new(repo: BoxBarcodeRepository) -> Self {
        Self { repo }
    }

    /// Generate a barcode image and store the record
    pub async fn generate_barcode(
        &self,
        tenant_id: i64,
        request: GenerateBarcodeRequest,
    ) -> Result<BarcodeConfig, ApiError> {
        request
            .validate()
            .map_err(|e| ApiError::Validation(e.join(", ")))?;

        let image_data = self
            .render_image(
                &request.barcode_type,
                &request.code,
                request.width,
                request.height,
            )
            .map_err(|e| ApiError::Internal(format!("Barcode generation failed: {}", e)))?;

        let create = CreateBarcode {
            entity_type: request.entity_type,
            entity_id: request.entity_type_id,
            barcode_type: request.barcode_type,
            code: request.code,
        };

        let mut barcode = self.repo.create(tenant_id, create).await?;
        barcode.image_data = Some(image_data);
        Ok(barcode)
    }

    /// Get barcode for an entity
    pub async fn get_barcode(
        &self,
        tenant_id: i64,
        entity_type: &str,
        entity_id: i64,
    ) -> Result<Option<BarcodeConfig>, ApiError> {
        self.repo
            .find_by_entity(tenant_id, entity_type, entity_id)
            .await
    }

    /// Delete a barcode by id
    pub async fn delete_barcode(&self, tenant_id: i64, id: i64) -> Result<(), ApiError> {
        self.repo.delete(id, tenant_id).await
    }

    /// List barcodes for a tenant with pagination
    pub async fn list_barcodes(
        &self,
        tenant_id: i64,
        page: u32,
        per_page: u32,
    ) -> Result<PaginatedResult<BarcodeConfig>, ApiError> {
        self.repo.find_by_tenant(tenant_id, page, per_page).await
    }

    /// Render a barcode/QR code image as base64 PNG
    fn render_image(
        &self,
        barcode_type: &BarcodeType,
        code: &str,
        width: Option<u32>,
        _height: Option<u32>,
    ) -> Result<String, Box<dyn std::error::Error>> {
        let _module_size = width.unwrap_or(200).clamp(50, 800);

        let gray_image = match barcode_type {
            BarcodeType::QrCode => {
                let qr = qrcode::QrCode::new(code.as_bytes())?;
                qr.render::<image::Luma<u8>>().build()
            }
            BarcodeType::Ean13 => {
                let output = barcodes::ean_upc::ean13::Ean13::encode(code)
                    .map_err(|e| format!("{:?}", e))?;
                output.to_image(2)
            }
            BarcodeType::Code128 => {
                let output = barcodes::linear::code128::Code128::encode(code)
                    .map_err(|e| format!("{:?}", e))?;
                output.to_image(2)
            }
            BarcodeType::DataMatrix => {
                let output = barcodes::twod::datamatrix::DataMatrix::encode(code)
                    .map_err(|e| format!("{:?}", e))?;
                output.to_image(4)
            }
        };

        let mut buffer = std::io::Cursor::new(Vec::new());
        let dynamic = image::DynamicImage::from(gray_image);
        dynamic.write_to(&mut buffer, image::ImageFormat::Png)?;

        use base64::Engine;
        let base64_str = base64::engine::general_purpose::STANDARD.encode(buffer.into_inner());
        Ok(format!("data:image/png;base64,{}", base64_str))
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use super::*;
    use crate::domain::barcode::repository::InMemoryBarcodeRepository;

    #[tokio::test]
    async fn test_generate_qr_code() {
        let repo = Arc::new(InMemoryBarcodeRepository::new()) as BoxBarcodeRepository;
        let service = BarcodeService::new(repo);

        let request = GenerateBarcodeRequest {
            entity_type: "invoice".to_string(),
            entity_type_id: 1,
            barcode_type: BarcodeType::QrCode,
            code: "https://example.com/invoice/1".to_string(),
            width: None,
            height: None,
        };

        let barcode = service.generate_barcode(1, request).await.unwrap();
        assert_eq!(barcode.barcode_type, BarcodeType::QrCode);
        assert!(barcode.image_data.is_some());
        assert!(barcode
            .image_data
            .unwrap()
            .starts_with("data:image/png;base64,"));
    }

    #[tokio::test]
    async fn test_generate_ean13() {
        let repo = Arc::new(InMemoryBarcodeRepository::new()) as BoxBarcodeRepository;
        let service = BarcodeService::new(repo);

        let request = GenerateBarcodeRequest {
            entity_type: "product".to_string(),
            entity_type_id: 1,
            barcode_type: BarcodeType::Ean13,
            code: "5901234123457".to_string(),
            width: None,
            height: None,
        };

        let barcode = service.generate_barcode(1, request).await.unwrap();
        assert_eq!(barcode.barcode_type, BarcodeType::Ean13);
        assert!(barcode.image_data.is_some());
    }

    #[tokio::test]
    async fn test_generate_code128() {
        let repo = Arc::new(InMemoryBarcodeRepository::new()) as BoxBarcodeRepository;
        let service = BarcodeService::new(repo);

        let request = GenerateBarcodeRequest {
            entity_type: "product".to_string(),
            entity_type_id: 1,
            barcode_type: BarcodeType::Code128,
            code: "ABC-123".to_string(),
            width: None,
            height: None,
        };

        let barcode = service.generate_barcode(1, request).await.unwrap();
        assert_eq!(barcode.barcode_type, BarcodeType::Code128);
        assert!(barcode.image_data.is_some());
    }

    #[tokio::test]
    async fn test_get_barcode() {
        let repo = Arc::new(InMemoryBarcodeRepository::new()) as BoxBarcodeRepository;
        let service = BarcodeService::new(repo.clone());

        let request = GenerateBarcodeRequest {
            entity_type: "product".to_string(),
            entity_type_id: 42,
            barcode_type: BarcodeType::QrCode,
            code: "TEST".to_string(),
            width: None,
            height: None,
        };
        service.generate_barcode(1, request).await.unwrap();

        let found = service.get_barcode(1, "product", 42).await.unwrap();
        assert!(found.is_some());
        assert_eq!(found.unwrap().entity_id, 42);
    }

    #[tokio::test]
    async fn test_delete_barcode() {
        let repo = Arc::new(InMemoryBarcodeRepository::new()) as BoxBarcodeRepository;
        let service = BarcodeService::new(repo.clone());

        let request = GenerateBarcodeRequest {
            entity_type: "product".to_string(),
            entity_type_id: 1,
            barcode_type: BarcodeType::QrCode,
            code: "TEST".to_string(),
            width: None,
            height: None,
        };
        let barcode = service.generate_barcode(1, request).await.unwrap();

        service.delete_barcode(1, barcode.id).await.unwrap();
        let found = repo.find_by_id(barcode.id, 1).await.unwrap();
        assert!(found.is_none());
    }

    #[tokio::test]
    async fn test_list_barcodes() {
        let repo = Arc::new(InMemoryBarcodeRepository::new()) as BoxBarcodeRepository;
        let service = BarcodeService::new(repo);

        for i in 1..=5 {
            let request = GenerateBarcodeRequest {
                entity_type: "product".to_string(),
                entity_type_id: i,
                barcode_type: BarcodeType::Code128,
                code: format!("CODE{}", i),
                width: None,
                height: None,
            };
            service.generate_barcode(1, request).await.unwrap();
        }

        let result = service.list_barcodes(1, 1, 2).await.unwrap();
        assert_eq!(result.items.len(), 2);
        assert_eq!(result.total, 5);
    }

    #[test]
    fn test_render_invalid_ean13() {
        let repo = Arc::new(InMemoryBarcodeRepository::new()) as BoxBarcodeRepository;
        let service = BarcodeService::new(repo);

        let result = service.render_image(&BarcodeType::Ean13, "abc", None, None);
        assert!(result.is_err());
    }
}
