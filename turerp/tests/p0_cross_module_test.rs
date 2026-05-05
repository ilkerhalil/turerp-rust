//! P0 Cross-Module Integration Tests
//!
//! Tests for the integration points between P0 modules:
//! - Invoice -> e-Fatura (EFaturaCreated event)
//! - Invoice -> Tax Engine (TaxPeriodCalculated event)
//! - Accounting -> e-Defter (EDefterPeriodCreated event, populate_from_accounting)
//! - Chart of Accounts -> Accounting (read-only dependency)
//!
//! Run with: cargo test --test p0_cross_module_test

use std::sync::Arc;
use turerp::common::events::EventSubscriber;
use turerp::common::{
    DomainEvent, EDefterAccountingSubscriber, EFaturaIntegrationSubscriber, EventBus,
    InMemoryEventBus, TaxPeriodSubscriber,
};
use turerp::domain::EFaturaStatus;

// ---- Domain Event tests ----

#[tokio::test]
async fn test_efatura_created_event_round_trip() {
    let bus = Arc::new(InMemoryEventBus::new());

    let event = DomainEvent::EFaturaCreated {
        tenant_id: 1,
        fatura_id: 100,
        uuid: "f47ac10b-58cc-4372-a567-0e02b2c3d479".to_string(),
    };

    let id = bus.publish(event.clone()).await.unwrap();
    assert!(id > 0);

    // Verify event_type and tenant_id
    assert_eq!(event.tenant_id(), 1);
    assert_eq!(event.event_type(), "efatura_created");
}

#[tokio::test]
async fn test_efatura_sent_event() {
    let bus = Arc::new(InMemoryEventBus::new());

    let event = DomainEvent::EFaturaSent {
        tenant_id: 2,
        fatura_id: 200,
        uuid: "abc-123".to_string(),
    };

    bus.publish(event).await.unwrap();
}

#[tokio::test]
async fn test_efatura_cancelled_event() {
    let bus = Arc::new(InMemoryEventBus::new());

    let event = DomainEvent::EFaturaCancelled {
        tenant_id: 3,
        fatura_id: 300,
        reason: "Fatura yanlis kesildi".to_string(),
    };

    bus.publish(event).await.unwrap();
}

#[tokio::test]
async fn test_edefter_period_created_event() {
    let bus = Arc::new(InMemoryEventBus::new());

    let event = DomainEvent::EDefterPeriodCreated {
        tenant_id: 1,
        period_id: 10,
        year: 2024,
        month: 6,
    };

    bus.publish(event).await.unwrap();
}

#[tokio::test]
async fn test_edefter_period_signed_and_sent_events() {
    let bus = Arc::new(InMemoryEventBus::new());

    let signed = DomainEvent::EDefterPeriodSigned {
        tenant_id: 1,
        period_id: 10,
    };
    let sent = DomainEvent::EDefterPeriodSent {
        tenant_id: 1,
        period_id: 10,
    };

    let ids = bus.publish_batch(vec![signed, sent]).await.unwrap();
    assert_eq!(ids.len(), 2);
}

#[tokio::test]
async fn test_tax_period_calculated_and_filed_events() {
    let bus = Arc::new(InMemoryEventBus::new());

    let calculated = DomainEvent::TaxPeriodCalculated {
        tenant_id: 5,
        period_id: 100,
        tax_type: "KDV".to_string(),
    };
    let filed = DomainEvent::TaxPeriodFiled {
        tenant_id: 5,
        period_id: 100,
        tax_type: "KDV".to_string(),
    };

    bus.publish_batch(vec![calculated, filed]).await.unwrap();
}

// ---- Subscriber integration tests ----

#[tokio::test]
async fn test_efatura_integration_subscriber_subscriptions() {
    let subscriber = EFaturaIntegrationSubscriber;
    assert_eq!(subscriber.name(), "EFaturaIntegrationSubscriber");
    let subscribed = subscriber.subscribed_to();
    assert!(subscribed.contains(&"invoice_created".to_string()));
    assert!(subscribed.contains(&"efatura_created".to_string()));
}

#[tokio::test]
async fn test_edefter_accounting_subscriber_subscriptions() {
    let subscriber = EDefterAccountingSubscriber;
    assert_eq!(subscriber.name(), "EDefterAccountingSubscriber");
    let subscribed = subscriber.subscribed_to();
    assert!(subscribed.contains(&"invoice_created".to_string()));
}

#[tokio::test]
async fn test_tax_period_subscriber_subscriptions() {
    let subscriber = TaxPeriodSubscriber;
    assert_eq!(subscriber.name(), "TaxPeriodSubscriber");
    let subscribed = subscriber.subscribed_to();
    assert!(subscribed.contains(&"tax_period_calculated".to_string()));
    assert!(subscribed.contains(&"tax_period_filed".to_string()));
}

#[tokio::test]
async fn test_cross_module_event_chain() {
    // Simulate a full P0 cross-module flow
    let bus = Arc::new(InMemoryEventBus::new());
    bus.subscribe(Arc::new(EFaturaIntegrationSubscriber))
        .await
        .unwrap();
    bus.subscribe(Arc::new(EDefterAccountingSubscriber))
        .await
        .unwrap();
    bus.subscribe(Arc::new(TaxPeriodSubscriber)).await.unwrap();

    let events = vec![
        DomainEvent::InvoiceCreated {
            invoice_id: 1,
            tenant_id: 1,
            amount: "10000.00".to_string(),
            currency: "TRY".to_string(),
        },
        DomainEvent::EFaturaCreated {
            tenant_id: 1,
            fatura_id: 1,
            uuid: "uuid-1".to_string(),
        },
        DomainEvent::EFaturaSent {
            tenant_id: 1,
            fatura_id: 1,
            uuid: "uuid-1".to_string(),
        },
        DomainEvent::TaxPeriodCalculated {
            tenant_id: 1,
            period_id: 1,
            tax_type: "KDV".to_string(),
        },
        DomainEvent::EDefterPeriodCreated {
            tenant_id: 1,
            period_id: 1,
            year: 2024,
            month: 6,
        },
        DomainEvent::EDefterPeriodSigned {
            tenant_id: 1,
            period_id: 1,
        },
        DomainEvent::EDefterPeriodSent {
            tenant_id: 1,
            period_id: 1,
        },
        DomainEvent::TaxPeriodFiled {
            tenant_id: 1,
            period_id: 1,
            tax_type: "KDV".to_string(),
        },
    ];

    let ids = bus.publish_batch(events).await.unwrap();
    assert_eq!(ids.len(), 8);
}

#[tokio::test]
async fn test_tenant_isolation_in_p0_events() {
    let e1 = DomainEvent::EFaturaCreated {
        tenant_id: 100,
        fatura_id: 1,
        uuid: "a".to_string(),
    };
    let e2 = DomainEvent::EDefterPeriodCreated {
        tenant_id: 200,
        period_id: 2,
        year: 2024,
        month: 1,
    };
    let e3 = DomainEvent::TaxPeriodCalculated {
        tenant_id: 300,
        period_id: 3,
        tax_type: "OIV".to_string(),
    };

    assert_eq!(e1.tenant_id(), 100);
    assert_eq!(e2.tenant_id(), 200);
    assert_eq!(e3.tenant_id(), 300);
}

#[tokio::test]
async fn test_all_new_event_types() {
    assert_eq!(
        DomainEvent::EFaturaCreated {
            tenant_id: 1,
            fatura_id: 1,
            uuid: "x".to_string(),
        }
        .event_type(),
        "efatura_created"
    );
    assert_eq!(
        DomainEvent::EFaturaSent {
            tenant_id: 1,
            fatura_id: 1,
            uuid: "x".to_string(),
        }
        .event_type(),
        "efatura_sent"
    );
    assert_eq!(
        DomainEvent::EFaturaCancelled {
            tenant_id: 1,
            fatura_id: 1,
            reason: "x".to_string(),
        }
        .event_type(),
        "efatura_cancelled"
    );
    assert_eq!(
        DomainEvent::EDefterPeriodCreated {
            tenant_id: 1,
            period_id: 1,
            year: 2024,
            month: 6,
        }
        .event_type(),
        "edefter_period_created"
    );
    assert_eq!(
        DomainEvent::EDefterPeriodSigned {
            tenant_id: 1,
            period_id: 1,
        }
        .event_type(),
        "edefter_period_signed"
    );
    assert_eq!(
        DomainEvent::EDefterPeriodSent {
            tenant_id: 1,
            period_id: 1,
        }
        .event_type(),
        "edefter_period_sent"
    );
    assert_eq!(
        DomainEvent::TaxPeriodCalculated {
            tenant_id: 1,
            period_id: 1,
            tax_type: "KDV".to_string(),
        }
        .event_type(),
        "tax_period_calculated"
    );
    assert_eq!(
        DomainEvent::TaxPeriodFiled {
            tenant_id: 1,
            period_id: 1,
            tax_type: "KDV".to_string(),
        }
        .event_type(),
        "tax_period_filed"
    );
}

// ---- e-Defter populate_from_accounting integration test ----

use rust_decimal::Decimal;
use turerp::domain::edefter::model::{
    CreateLedgerPeriod, EDefterStatus, LedgerType, YevmiyeEntry, YevmiyeLine,
};
use turerp::domain::edefter::repository::InMemoryEDefterRepository;
use turerp::domain::edefter::service::EDefterService;

fn make_edefter_service() -> EDefterService {
    let repo = Arc::new(InMemoryEDefterRepository::new());
    EDefterService::new(repo)
}

#[tokio::test]
async fn test_edefter_populate_from_accounting_flow() {
    let svc = make_edefter_service();

    // Create a period
    let period = svc
        .create_period(
            CreateLedgerPeriod {
                year: 2024,
                month: 6,
                period_type: LedgerType::YevmiyeDefteri,
            },
            1,
        )
        .await
        .unwrap();
    assert_eq!(period.status, EDefterStatus::Draft);

    // Populate from accounting (cross-module: Accounting -> e-Defter)
    let entries = vec![YevmiyeEntry {
        id: 0,
        period_id: period.id,
        entry_number: 1,
        entry_date: chrono::NaiveDate::from_ymd_opt(2024, 6, 1).unwrap(),
        explanation: "Satis kaydi".to_string(),
        debit_total: Decimal::new(10000, 2),
        credit_total: Decimal::new(10000, 2),
        lines: vec![
            YevmiyeLine {
                account_code: "100.01".to_string(),
                account_name: "Kasa".to_string(),
                debit: Decimal::new(10000, 2),
                credit: Decimal::ZERO,
                explanation: "Kasa borc".to_string(),
            },
            YevmiyeLine {
                account_code: "600.01".to_string(),
                account_name: "Satis Geliri".to_string(),
                debit: Decimal::ZERO,
                credit: Decimal::new(10000, 2),
                explanation: "Satis alacak".to_string(),
            },
        ],
    }];

    let populated = svc
        .populate_from_accounting(period.id, 1, entries)
        .await
        .unwrap();
    assert_eq!(populated.len(), 1);
    assert_eq!(populated[0].explanation, "Satis kaydi");

    // Validate balance
    let balance = svc.validate_balance(period.id, 1).await.unwrap();
    assert!(balance.is_balanced);
    assert_eq!(balance.total_debit, Decimal::new(10000, 2));
    assert_eq!(balance.total_credit, Decimal::new(10000, 2));
}

#[tokio::test]
async fn test_edefter_populate_rejects_non_draft_period() {
    let svc = make_edefter_service();

    let period = svc
        .create_period(
            CreateLedgerPeriod {
                year: 2024,
                month: 6,
                period_type: LedgerType::YevmiyeDefteri,
            },
            1,
        )
        .await
        .unwrap();

    // Sign the period so it is no longer Draft
    svc.sign_berat(period.id, 1).await.unwrap();

    let entries = vec![YevmiyeEntry {
        id: 0,
        period_id: period.id,
        entry_number: 1,
        entry_date: chrono::NaiveDate::from_ymd_opt(2024, 6, 1).unwrap(),
        explanation: "Should fail".to_string(),
        debit_total: Decimal::ZERO,
        credit_total: Decimal::ZERO,
        lines: vec![],
    }];

    let result = svc.populate_from_accounting(period.id, 1, entries).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_edefter_populate_invalid_period() {
    let svc = make_edefter_service();

    let entries = vec![YevmiyeEntry {
        id: 0,
        period_id: 9999,
        entry_number: 1,
        entry_date: chrono::NaiveDate::from_ymd_opt(2024, 6, 1).unwrap(),
        explanation: "Invalid".to_string(),
        debit_total: Decimal::ZERO,
        credit_total: Decimal::ZERO,
        lines: vec![],
    }];

    let result = svc.populate_from_accounting(9999, 1, entries).await;
    assert!(result.is_err());
}

// ---- e-Fatura -> Tax Engine integration test ----

use turerp::common::gov::InMemoryGibGateway;
use turerp::domain::efatura::model::EFaturaProfile;
use turerp::domain::efatura::repository::InMemoryEFaturaRepository;
use turerp::domain::efatura::service::EFaturaService;

fn make_efatura_service() -> EFaturaService {
    let repo = Arc::new(InMemoryEFaturaRepository::new());
    let gateway = Arc::new(InMemoryGibGateway::new());
    EFaturaService::new(repo, gateway)
}

#[tokio::test]
async fn test_efatura_create_from_invoice_and_send() {
    let svc = make_efatura_service();

    // Create an e-Fatura draft from an invoice (cross-module: Invoice -> e-Fatura)
    let fatura = svc
        .create_from_invoice(42, EFaturaProfile::TemelFatura, 1)
        .await
        .unwrap();
    assert_eq!(fatura.invoice_id, Some(42));
    assert_eq!(fatura.status, EFaturaStatus::Draft);

    // Send to GIB
    let sent = svc.send_to_gib(fatura.id, 1).await.unwrap();
    assert_eq!(sent.status, EFaturaStatus::Sent);

    // Check status at GIB
    let checked = svc.check_status(&sent.uuid, 1).await.unwrap();
    assert_eq!(checked.status, EFaturaStatus::Accepted);
}

#[tokio::test]
async fn test_efatura_cancel_flow() {
    let svc = make_efatura_service();

    let fatura = svc
        .create_from_invoice(1, EFaturaProfile::TemelFatura, 1)
        .await
        .unwrap();

    svc.send_to_gib(fatura.id, 1).await.unwrap();

    // Cancel (cross-module: e-Fatura status lifecycle)
    let cancelled = svc
        .cancel_efatura(fatura.id, 1, "Iptal nedeni".to_string())
        .await
        .unwrap();
    assert_eq!(cancelled.status, EFaturaStatus::Cancelled);
}

// ---- Tax Engine period lifecycle integration test ----

use chrono::NaiveDate;
use turerp::domain::tax::model::{CreateTaxPeriod, CreateTaxRate, TaxPeriodStatus, TaxType};
use turerp::domain::tax::repository::{InMemoryTaxPeriodRepository, InMemoryTaxRateRepository};
use turerp::domain::tax::service::TaxService;

fn make_tax_service() -> TaxService {
    let rate_repo = Arc::new(InMemoryTaxRateRepository::new());
    let period_repo = Arc::new(InMemoryTaxPeriodRepository::new());
    TaxService::new(rate_repo, period_repo)
}

#[tokio::test]
async fn test_tax_period_lifecycle_with_events() {
    let svc = make_tax_service();

    // Create a tax rate (KDV 20%)
    let rate = svc
        .create_tax_rate(
            CreateTaxRate {
                tax_type: TaxType::KDV,
                rate: Decimal::new(20, 2),
                effective_from: NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
                effective_to: None,
                category: None,
                description: "Standard KDV".to_string(),
                is_default: true,
            },
            1,
        )
        .await
        .unwrap();
    assert_eq!(rate.tax_type, TaxType::KDV);

    // Calculate tax for an invoice amount (cross-module: Invoice -> Tax Engine)
    let result = svc
        .calculate_tax(
            TaxType::KDV,
            Decimal::new(10000, 2),
            NaiveDate::from_ymd_opt(2024, 6, 15).unwrap(),
            1,
            false,
        )
        .await
        .unwrap();

    assert_eq!(result.base_amount, Decimal::new(10000, 2));
    assert_eq!(result.tax_amount, Decimal::new(2000, 2));

    // Create a tax period and calculate it
    let period = svc
        .create_tax_period(
            CreateTaxPeriod {
                tax_type: TaxType::KDV,
                period_year: 2024,
                period_month: 6,
            },
            1,
        )
        .await
        .unwrap();
    assert_eq!(period.status, TaxPeriodStatus::Open);

    // Calculate period (cross-module: Tax Engine -> e-Fatura)
    let calculated = svc.calculate_period(period.id, 1).await.unwrap();
    assert_eq!(calculated.status, TaxPeriodStatus::Calculated);

    // File period
    let filed = svc.file_period(period.id, 1).await.unwrap();
    assert_eq!(filed.status, TaxPeriodStatus::Filed);
}

// ---- Chart of Accounts -> Accounting integration test ----

use rust_decimal_macros::dec;
use turerp::domain::accounting::model::{
    AccountSubType, AccountType, CreateAccount, CreateJournalEntry, CreateJournalLine,
};
use turerp::domain::accounting::repository::{
    InMemoryAccountRepository, InMemoryJournalEntryRepository, InMemoryJournalLineRepository,
};
use turerp::domain::accounting::service::AccountingService;
use turerp::domain::chart_of_accounts::model::AccountGroup;
use turerp::domain::chart_of_accounts::repository::InMemoryChartAccountRepository;
use turerp::domain::chart_of_accounts::service::ChartOfAccountsService;
use turerp::domain::{BoxAccountRepository, BoxJournalEntryRepository, BoxJournalLineRepository};

#[tokio::test]
async fn test_chart_of_accounts_to_accounting_flow() {
    // Create Chart of Accounts service (cross-module: CoA -> Accounting)
    let chart_repo = Arc::new(InMemoryChartAccountRepository::new());
    let chart_service = ChartOfAccountsService::new(chart_repo);

    // Create accounts in Chart of Accounts
    let cash_account = chart_service
        .create_account(
            turerp::domain::chart_of_accounts::model::CreateChartAccount {
                code: "100.01".to_string(),
                name: "Kasa".to_string(),
                group: AccountGroup::DonenVarliklar,
                parent_code: None,
                account_type: AccountType::Asset,
                allow_posting: true,
            },
            1,
        )
        .await
        .unwrap();

    let sales_account = chart_service
        .create_account(
            turerp::domain::chart_of_accounts::model::CreateChartAccount {
                code: "600.01".to_string(),
                name: "Satis Geliri".to_string(),
                group: AccountGroup::GelirTablosu,
                parent_code: None,
                account_type: AccountType::Revenue,
                allow_posting: true,
            },
            1,
        )
        .await
        .unwrap();

    // Get the trial balance from Chart of Accounts
    let trial_balance = chart_service.get_trial_balance(1).await.unwrap();
    assert_eq!(trial_balance.len(), 2);

    // Create an accounting entry using those account codes
    // (cross-module: CoA provides codes -> Accounting uses them)
    let account_repo = Arc::new(InMemoryAccountRepository::new()) as BoxAccountRepository;
    let entry_repo = Arc::new(InMemoryJournalEntryRepository::new()) as BoxJournalEntryRepository;
    let line_repo = Arc::new(InMemoryJournalLineRepository::new()) as BoxJournalLineRepository;
    let accounting_service = AccountingService::new(account_repo, entry_repo, line_repo);

    // Create accounts in the accounting system matching the chart
    accounting_service
        .create_account(CreateAccount {
            tenant_id: 1,
            code: "100.01".to_string(),
            name: "Kasa".to_string(),
            account_type: AccountType::Asset,
            sub_type: AccountSubType::CurrentAsset,
            parent_id: Some(cash_account.id),
            allow_transaction: true,
        })
        .await
        .unwrap();

    accounting_service
        .create_account(CreateAccount {
            tenant_id: 1,
            code: "600.01".to_string(),
            name: "Satis Geliri".to_string(),
            account_type: AccountType::Revenue,
            sub_type: AccountSubType::OperatingRevenue,
            parent_id: Some(sales_account.id),
            allow_transaction: true,
        })
        .await
        .unwrap();

    // Create a journal entry (cross-module: Accounting -> e-Defter)
    let entry = accounting_service
        .create_journal_entry(CreateJournalEntry {
            tenant_id: 1,
            date: chrono::Utc::now(),
            description: "Satis kaydi".to_string(),
            reference: Some("INV-001".to_string()),
            created_by: 1,
            lines: vec![
                CreateJournalLine {
                    account_id: 1,
                    debit: dec!(1000.0),
                    credit: Decimal::ZERO,
                    description: None,
                    reference: None,
                },
                CreateJournalLine {
                    account_id: 2,
                    debit: Decimal::ZERO,
                    credit: dec!(1000.0),
                    description: None,
                    reference: None,
                },
            ],
        })
        .await
        .unwrap();

    assert_eq!(entry.tenant_id, 1);
    assert_eq!(entry.description, "Satis kaydi");
}
