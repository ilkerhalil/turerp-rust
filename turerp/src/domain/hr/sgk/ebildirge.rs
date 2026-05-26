//! e-Bildirge XML generator for Turkish SGK monthly declarations

use crate::domain::hr::model::Employee;
use crate::domain::hr::sgk::model::SgkPayrollLineItem;
use rust_decimal::Decimal;

#[derive(Clone, Debug, serde::Deserialize, serde::Serialize)]
pub struct EmployerInfo {
    pub company_name: String,
    pub tax_number: String,
    pub sgk_workplace_code: String,
    pub address: String,
    pub phone: String,
}

pub struct EBildirgeGenerator;

impl EBildirgeGenerator {
    pub fn generate_monthly_declaration(
        employer_info: &EmployerInfo,
        period_year: i32,
        period_month: i32,
        employees: &[(Employee, SgkPayrollLineItem)],
    ) -> String {
        let mut xml = String::new();
        xml.push_str("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n");
        xml.push_str("<AylikPrimHizmetBelgesi xmlns=\"http://www.sgk.gov.tr/...\">\n");

        xml.push_str("  <IsyeriBilgileri>\n");
        xml.push_str(&format!(
            "    <IsyeriSicilNo>{}</IsyeriSicilNo>\n",
            xml_escape(&employer_info.sgk_workplace_code)
        ));
        xml.push_str(&format!(
            "    <IsyeriAdi>{}</IsyeriAdi>\n",
            xml_escape(&employer_info.company_name)
        ));
        xml.push_str(&format!(
            "    <IsyeriAdresi>{}</IsyeriAdresi>\n",
            xml_escape(&employer_info.address)
        ));
        xml.push_str(&format!(
            "    <IsyeriTelefon>{}</IsyeriTelefon>\n",
            xml_escape(&employer_info.phone)
        ));
        xml.push_str("  </IsyeriBilgileri>\n");

        xml.push_str("  <Donem>\n");
        xml.push_str(&format!("    <Yil>{}</Yil>\n", period_year));
        xml.push_str(&format!("    <Ay>{}</Ay>\n", period_month));
        xml.push_str("  </Donem>\n");

        xml.push_str("  <Calisanlar>\n");

        let mut total_base = Decimal::ZERO;
        let mut total_worker = Decimal::ZERO;
        let mut total_employer = Decimal::ZERO;

        for (employee, item) in employees {
            let employer_sgk = item.sgk_earnings_base * Decimal::new(205, 3);
            let employer_unemployment = item.sgk_earnings_base * Decimal::new(2, 2);
            let employer_total = employer_sgk + employer_unemployment;

            total_base += item.sgk_earnings_base;
            total_worker += item.sgk_premium_worker + item.unemployment_premium_worker;
            total_employer += employer_total;

            xml.push_str("    <Calisan>\n");

            xml.push_str("      <KimlikBilgileri>\n");
            xml.push_str(&format!(
                "        <TCKimlikNo>{}</TCKimlikNo>\n",
                xml_escape(&employee.tc_kimlik_no)
            ));
            xml.push_str(&format!(
                "        <SicilNo>{}</SicilNo>\n",
                xml_escape(
                    employee
                        .sgk_sicil_no
                        .as_deref()
                        .unwrap_or(&employee.employee_number)
                )
            ));
            xml.push_str(&format!(
                "        <Ad>{}</Ad>\n",
                xml_escape(&employee.first_name)
            ));
            xml.push_str(&format!(
                "        <Soyad>{}</Soyad>\n",
                xml_escape(&employee.last_name)
            ));
            xml.push_str("      </KimlikBilgileri>\n");

            xml.push_str("      <PrimBilgileri>\n");
            xml.push_str("        <EksikGun>0</EksikGun>\n");
            xml.push_str("        <Kazanclar>\n");
            xml.push_str(&format!(
                "          <ToplamKazanc>{}</ToplamKazanc>\n",
                item.gross_salary
            ));
            xml.push_str(&format!(
                "          <SgkMatrah>{}</SgkMatrah>\n",
                item.sgk_earnings_base
            ));
            xml.push_str("        </Kazanclar>\n");
            xml.push_str("        <Primler>\n");
            xml.push_str(&format!(
                "          <IsverenPayi>{}</IsverenPayi>\n",
                employer_total
            ));
            xml.push_str(&format!(
                "          <IsciPayi>{}</IsciPayi>\n",
                item.sgk_premium_worker
            ));
            xml.push_str(&format!(
                "          <IssizlikIsverenPayi>{}</IssizlikIsverenPayi>\n",
                employer_unemployment
            ));
            xml.push_str(&format!(
                "          <IssizlikIsciPayi>{}</IssizlikIsciPayi>\n",
                item.unemployment_premium_worker
            ));
            xml.push_str("        </Primler>\n");
            xml.push_str("      </PrimBilgileri>\n");

            xml.push_str("    </Calisan>\n");
        }

        xml.push_str("  </Calisanlar>\n");

        xml.push_str("  <Toplamlar>\n");
        xml.push_str(&format!(
            "    <CalisanSayisi>{}</CalisanSayisi>\n",
            employees.len()
        ));
        xml.push_str(&format!(
            "    <ToplamMatrah>{}</ToplamMatrah>\n",
            total_base
        ));
        xml.push_str(&format!(
            "    <ToplamIsciPayi>{}</ToplamIsciPayi>\n",
            total_worker
        ));
        xml.push_str(&format!(
            "    <ToplamIsverenPayi>{}</ToplamIsverenPayi>\n",
            total_employer
        ));
        xml.push_str("  </Toplamlar>\n");

        xml.push_str("</AylikPrimHizmetBelgesi>");
        xml
    }
}

fn xml_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::hr::model::EmployeeStatus;
    use chrono::Utc;
    use rust_decimal::Decimal;

    fn sample_employee() -> Employee {
        Employee {
            id: 1,
            tenant_id: 1,
            company_id: 1,
            user_id: None,
            employee_number: "EMP001".to_string(),
            first_name: "John".to_string(),
            last_name: "Doe".to_string(),
            email: "john@example.com".to_string(),
            phone: None,
            department: None,
            position: None,
            hire_date: Utc::now(),
            termination_date: None,
            status: EmployeeStatus::Active,
            salary: Decimal::new(100000, 2),
            tc_kimlik_no: "12345678901".to_string(),
            iban: None,
            sgk_sicil_no: Some("SGK001".to_string()),
            marital_status: None,
            children_count: 0,
            spouse_working: false,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            deleted_at: None,
            deleted_by: None,
        }
    }

    fn sample_line_item() -> SgkPayrollLineItem {
        SgkPayrollLineItem {
            employee_id: 1,
            gross_salary: Decimal::new(100000, 2),
            sgk_earnings_base: Decimal::new(100000, 2),
            sgk_premium_worker: Decimal::new(14000, 2),
            unemployment_premium_worker: Decimal::new(1000, 2),
            income_tax_base: Decimal::new(85000, 2),
            income_tax: Decimal::new(12750, 2),
            stamp_tax: Decimal::new(759, 2),
            agi: Decimal::new(2800, 2),
            net_salary: Decimal::new(74591, 2),
            employer_cost: Decimal::new(122500, 2),
        }
    }

    #[test]
    fn test_generate_monthly_declaration_basic() {
        let employer = EmployerInfo {
            company_name: "Test Co".to_string(),
            tax_number: "1234567890".to_string(),
            sgk_workplace_code: "WP001".to_string(),
            address: "Test Address".to_string(),
            phone: "5551234567".to_string(),
        };
        let emp = sample_employee();
        let item = sample_line_item();
        let xml =
            EBildirgeGenerator::generate_monthly_declaration(&employer, 2024, 6, &[(emp, item)]);

        assert!(xml.contains("<?xml version=\"1.0\" encoding=\"UTF-8\"?>"));
        assert!(xml.contains("<AylikPrimHizmetBelgesi xmlns=\"http://www.sgk.gov.tr/...\">"));
        assert!(xml.contains("<IsyeriSicilNo>WP001</IsyeriSicilNo>"));
        assert!(xml.contains("<IsyeriAdi>Test Co</IsyeriAdi>"));
        assert!(xml.contains("<Yil>2024</Yil>"));
        assert!(xml.contains("<Ay>6</Ay>"));
        assert!(xml.contains("<TCKimlikNo>12345678901</TCKimlikNo>"));
        assert!(xml.contains("<SicilNo>SGK001</SicilNo>"));
        assert!(xml.contains("<Ad>John</Ad>"));
        assert!(xml.contains("<Soyad>Doe</Soyad>"));
        assert!(xml.contains("<EksikGun>0</EksikGun>"));
        assert!(xml.contains("<ToplamKazanc>1000.00</ToplamKazanc>"));
        assert!(xml.contains("<SgkMatrah>1000.00</SgkMatrah>"));
        assert!(xml.contains("<CalisanSayisi>1</CalisanSayisi>"));
        assert!(xml.contains("<Toplamlar>"));
    }

    #[test]
    fn test_generate_monthly_declaration_xml_escape() {
        let employer = EmployerInfo {
            company_name: "A & B Co".to_string(),
            tax_number: "123".to_string(),
            sgk_workplace_code: "WP001".to_string(),
            address: "1 < 2 & 2 > 1".to_string(),
            phone: "555".to_string(),
        };
        let mut emp = sample_employee();
        emp.first_name = "Tom < & >".to_string();
        emp.tc_kimlik_no = "1&2'3".to_string();
        let item = sample_line_item();
        let xml =
            EBildirgeGenerator::generate_monthly_declaration(&employer, 2024, 6, &[(emp, item)]);

        assert!(xml.contains("A &amp; B Co"));
        assert!(xml.contains("1 &lt; 2 &amp; 2 &gt; 1"));
        assert!(xml.contains("Tom &lt; &amp; &gt;"));
        assert!(xml.contains("1&amp;2&apos;3"));
    }

    #[test]
    fn test_generate_monthly_declaration_fallback_sicil() {
        let employer = EmployerInfo {
            company_name: "Co".to_string(),
            tax_number: "123".to_string(),
            sgk_workplace_code: "WP".to_string(),
            address: "Addr".to_string(),
            phone: "555".to_string(),
        };
        let mut emp = sample_employee();
        emp.sgk_sicil_no = None;
        let item = sample_line_item();
        let xml =
            EBildirgeGenerator::generate_monthly_declaration(&employer, 2024, 6, &[(emp, item)]);

        assert!(xml.contains("<SicilNo>EMP001</SicilNo>"));
    }

    #[test]
    fn test_generate_monthly_declaration_multiple_employees() {
        let employer = EmployerInfo {
            company_name: "Co".to_string(),
            tax_number: "123".to_string(),
            sgk_workplace_code: "WP".to_string(),
            address: "Addr".to_string(),
            phone: "555".to_string(),
        };
        let emp1 = sample_employee();
        let mut emp2 = sample_employee();
        emp2.id = 2;
        emp2.employee_number = "EMP002".to_string();
        emp2.first_name = "Jane".to_string();
        emp2.last_name = "Smith".to_string();
        emp2.tc_kimlik_no = "98765432109".to_string();
        let item1 = sample_line_item();
        let mut item2 = sample_line_item();
        item2.employee_id = 2;
        let xml = EBildirgeGenerator::generate_monthly_declaration(
            &employer,
            2024,
            6,
            &[(emp1, item1), (emp2, item2)],
        );

        assert!(xml.contains("<CalisanSayisi>2</CalisanSayisi>"));
        assert!(xml.contains("<Ad>John</Ad>"));
        assert!(xml.contains("<Ad>Jane</Ad>"));
    }
}
