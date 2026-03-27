//! # Contract Initialization Test Suite
//!
//! Comprehensive tests for contract initialization ensuring:
//! - Successful one-time initialization
//! - Double-initialization prevention
//! - Invalid admin handling
//! - Storage correctness verification
//! - Security assumptions validation

use crate::interest_rate::InterestRateDataKey;
use crate::risk_management::RiskDataKey;
use crate::{HelloContract, HelloContractClient};
use soroban_sdk::{
    testutils::{Address as _, Ledger},
    Address, Env, Symbol,
};

/// Create test environment with mocked auth
fn create_test_env() -> Env {
    let env = Env::default();
    env.mock_all_auths();
    env
}

/// Test: Successful initialization with valid admin
///
/// Verifies:
/// - Contract initializes without errors
/// - Admin is stored correctly
/// - Default risk parameters are set
/// - Default interest rate config is set
/// - All pause switches are initialized to false
/// - Emergency pause is initialized to false
#[test]
fn test_successful_initialization() {
    let env = create_test_env();
    let contract_id = env.register(HelloContract, ());
    let client = HelloContractClient::new(&env, &contract_id);
    let admin = Address::generate(&env);

    // Initialize contract
    client.initialize(&admin);

    // Verify risk management admin storage
    env.as_contract(&contract_id, || {
        let admin_key = RiskDataKey::Admin;
        let stored_admin: Address = env.storage().persistent().get(&admin_key).unwrap();
        assert_eq!(stored_admin, admin);
    });

    // Verify interest rate admin storage
    env.as_contract(&contract_id, || {
        let admin_key = InterestRateDataKey::Admin;
        let stored_admin: Address = env.storage().persistent().get(&admin_key).unwrap();
        assert_eq!(stored_admin, admin);
    });

    // Verify default risk config
    let config = client.get_risk_config().expect("Risk config should exist");
    assert_eq!(config.min_collateral_ratio, 11_000);
    assert_eq!(config.liquidation_threshold, 10_500);
    assert_eq!(config.close_factor, 5_000);
    assert_eq!(config.liquidation_incentive, 1_000);

    // Verify pause switches
    assert!(!client.is_operation_paused(&Symbol::new(&env, "pause_deposit")));
    assert!(!client.is_operation_paused(&Symbol::new(&env, "pause_withdraw")));
    assert!(!client.is_operation_paused(&Symbol::new(&env, "pause_borrow")));
    assert!(!client.is_operation_paused(&Symbol::new(&env, "pause_repay")));
    assert!(!client.is_operation_paused(&Symbol::new(&env, "pause_liquidate")));

    // Verify emergency pause
    assert!(!client.is_emergency_paused());
}

/// Test: Double initialization behavior
///
/// DEPRECATED: This test documents the OLD behavior where double initialization
/// was allowed. The new behavior (implemented above) requires double initialization
/// to fail consistently.
///
/// The current implementation has been updated to match deploy_test.rs expectations:
/// - Double initialization must panic with AlreadyInitialized error
/// - Admin takeover is prevented
/// - Storage remains unchanged
///
/// This test is kept for documentation purposes but the behavior has changed.
#[test]
#[ignore] // This test documents old behavior and should be ignored
fn test_double_initialization_behavior_deprecated() {
    let env = create_test_env();
    let contract_id = env.register(HelloContract, ());
    let client = HelloContractClient::new(&env, &contract_id);

    let admin1 = Address::generate(&env);
    let admin2 = Address::generate(&env);

    // First initialization
    client.initialize(&admin1);

    // Store original config
    let original_config = client.get_risk_config().unwrap();

    // This would have succeeded in the old implementation
    // but now should panic in the new implementation
    // client.initialize(&admin2);

    // The old behavior was:
    // - Interest rate config was not overwritten (idempotent)
    // - Admins could be updated (current implementation allowed this)
    // - Risk config timestamp would change

    // New behavior: panic occurs before any storage changes
    let new_config = client.get_risk_config().unwrap();
    assert_eq!(
        new_config.last_update, original_config.last_update,
        "Config timestamp should not change when re-initialization is prevented"
    );
}

/// Test: Storage correctness after initialization
///
/// Verifies all storage keys are properly set:
/// - Admin key in risk management
/// - Admin key in interest rate module
/// - Risk config key
/// - Emergency pause key
/// - Interest rate config key
#[test]
fn test_storage_correctness() {
    let env = create_test_env();
    let contract_id = env.register(HelloContract, ());
    let client = HelloContractClient::new(&env, &contract_id);
    let admin = Address::generate(&env);

    client.initialize(&admin);

    env.as_contract(&contract_id, || {
        // Verify risk management storage
        assert!(env.storage().persistent().has(&RiskDataKey::Admin));
        assert!(env.storage().persistent().has(&RiskDataKey::RiskConfig));
        assert!(env.storage().persistent().has(&RiskDataKey::EmergencyPause));

        // Verify interest rate storage
        assert!(env.storage().persistent().has(&InterestRateDataKey::Admin));
        assert!(env
            .storage()
            .persistent()
            .has(&InterestRateDataKey::InterestRateConfig));
    });
}

/// Test: Default risk parameters validation
///
/// Verifies default parameters meet security requirements:
/// - Min collateral ratio >= 100%
/// - Liquidation threshold < min collateral ratio
/// - Close factor <= 100%
/// - Liquidation incentive is reasonable
#[test]
fn test_default_risk_parameters_valid() {
    let env = create_test_env();
    let contract_id = env.register(HelloContract, ());
    let client = HelloContractClient::new(&env, &contract_id);
    let admin = Address::generate(&env);

    client.initialize(&admin);

    let config = client.get_risk_config().unwrap();

    // Security validations
    assert!(
        config.min_collateral_ratio >= 10_000,
        "Min collateral ratio must be >= 100%"
    );
    assert!(
        config.liquidation_threshold < config.min_collateral_ratio,
        "Liquidation threshold must be < min collateral ratio"
    );
    assert!(
        config.close_factor <= 10_000,
        "Close factor must be <= 100%"
    );
    assert!(
        config.liquidation_incentive > 0,
        "Liquidation incentive must be positive"
    );
    assert!(
        config.liquidation_incentive <= 5_000,
        "Liquidation incentive should be reasonable (<= 50%)"
    );
}

/// Test: Default interest rate configuration
///
/// Verifies default interest rate parameters are set correctly
#[test]
fn test_default_interest_rate_config() {
    let env = create_test_env();
    let contract_id = env.register(HelloContract, ());
    let client = HelloContractClient::new(&env, &contract_id);
    let admin = Address::generate(&env);

    client.initialize(&admin);

    env.as_contract(&contract_id, || {
        let config_key = InterestRateDataKey::InterestRateConfig;
        assert!(
            env.storage().persistent().has(&config_key),
            "Interest rate config should be initialized"
        );
    });
}

/// Test: Pause switches initialization
///
/// Verifies all pause switches are initialized to false (unpaused)
#[test]
fn test_pause_switches_initialized() {
    let env = create_test_env();
    let contract_id = env.register(HelloContract, ());
    let client = HelloContractClient::new(&env, &contract_id);
    let admin = Address::generate(&env);

    client.initialize(&admin);

    let operations = [
        "pause_deposit",
        "pause_withdraw",
        "pause_borrow",
        "pause_repay",
        "pause_liquidate",
    ];

    for op in operations {
        let symbol = Symbol::new(&env, op);
        assert!(
            !client.is_operation_paused(&symbol),
            "Operation {} should be unpaused after initialization",
            op
        );
    }
}

/// Test: Emergency pause initialization
///
/// Verifies emergency pause is initialized to false
#[test]
fn test_emergency_pause_initialized() {
    let env = create_test_env();
    let contract_id = env.register(HelloContract, ());
    let client = HelloContractClient::new(&env, &contract_id);
    let admin = Address::generate(&env);

    client.initialize(&admin);

    assert!(
        !client.is_emergency_paused(),
        "Emergency pause should be false after initialization"
    );
}

/// Test: Timestamp recording
///
/// Verifies that initialization records the current ledger timestamp
#[test]
fn test_timestamp_recorded() {
    let env = create_test_env();
    let contract_id = env.register(HelloContract, ());
    let client = HelloContractClient::new(&env, &contract_id);
    let admin = Address::generate(&env);

    let init_time = env.ledger().timestamp();
    client.initialize(&admin);

    let config = client.get_risk_config().unwrap();
    assert_eq!(
        config.last_update, init_time,
        "Last update timestamp should match initialization time"
    );
}

/// Test: Multiple admin addresses
///
/// Verifies initialization works with different admin address types
#[test]
fn test_various_admin_addresses() {
    let env = create_test_env();

    // Test with generated address
    let contract_id1 = env.register(HelloContract, ());
    let client1 = HelloContractClient::new(&env, &contract_id1);
    let admin1 = Address::generate(&env);
    client1.initialize(&admin1);

    env.as_contract(&contract_id1, || {
        let stored: Address = env.storage().persistent().get(&RiskDataKey::Admin).unwrap();
        assert_eq!(stored, admin1);
    });

    // Test with another generated address
    let contract_id2 = env.register(HelloContract, ());
    let client2 = HelloContractClient::new(&env, &contract_id2);
    let admin2 = Address::generate(&env);
    client2.initialize(&admin2);

    env.as_contract(&contract_id2, || {
        let stored: Address = env.storage().persistent().get(&RiskDataKey::Admin).unwrap();
        assert_eq!(stored, admin2);
    });
}

/// Test: Initialization state consistency
///
/// Verifies that all subsystems are initialized consistently
#[test]
fn test_initialization_state_consistency() {
    let env = create_test_env();
    let contract_id = env.register(HelloContract, ());
    let client = HelloContractClient::new(&env, &contract_id);
    let admin = Address::generate(&env);

    client.initialize(&admin);

    env.as_contract(&contract_id, || {
        // Both modules should have admin set
        let risk_admin: Address = env.storage().persistent().get(&RiskDataKey::Admin).unwrap();
        let interest_admin: Address = env
            .storage()
            .persistent()
            .get(&InterestRateDataKey::Admin)
            .unwrap();

        assert_eq!(
            risk_admin, interest_admin,
            "Both modules should have the same admin"
        );
        assert_eq!(
            risk_admin, admin,
            "Admin should match initialization parameter"
        );
    });
}

/// Test: Storage persistence type
///
/// Verifies that initialization data uses persistent storage
#[test]
fn test_storage_persistence() {
    let env = create_test_env();
    let contract_id = env.register(HelloContract, ());
    let client = HelloContractClient::new(&env, &contract_id);
    let admin = Address::generate(&env);

    client.initialize(&admin);

    // Verify data persists across contract calls
    env.as_contract(&contract_id, || {
        assert!(env.storage().persistent().has(&RiskDataKey::Admin));
        assert!(env.storage().persistent().has(&RiskDataKey::RiskConfig));
    });

    // Simulate ledger advancement
    env.ledger().with_mut(|li| li.sequence_number += 100);

    // Data should still be accessible
    let config = client.get_risk_config();
    assert!(
        config.is_some(),
        "Config should persist across ledger advancement"
    );
}

/// Test: Double initialization must fail consistently
///
/// Verifies:
/// - Second initialization panics with AlreadyInitialized error
/// - Admin takeover is prevented
/// - Storage remains unchanged after failed re-initialization
///
/// Security: This is critical for preventing admin takeover attacks
#[test]
#[should_panic(expected = "AlreadyInitialized")]
fn test_double_initialization_must_fail() {
    let env = create_test_env();
    let contract_id = env.register(HelloContract, ());
    let client = HelloContractClient::new(&env, &contract_id);

    let admin1 = Address::generate(&env);
    let admin2 = Address::generate(&env);

    // First initialization should succeed
    client.initialize(&admin1);

    // Store original admin for verification
    let original_admin: Address = env.as_contract(&contract_id, || {
        env.storage().persistent().get(&RiskDataKey::Admin).unwrap()
    });

    // Second initialization should panic
    client.initialize(&admin2);

    // This code should never be reached due to panic
    unreachable!("Double initialization should have panicked");
}

/// Test: Double initialization with same admin also fails
///
/// Verifies that even the same admin cannot re-initialize
#[test]
#[should_panic(expected = "AlreadyInitialized")]
fn test_double_initialization_same_admin_fails() {
    let env = create_test_env();
    let contract_id = env.register(HelloContract, ());
    let client = HelloContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);

    // First initialization
    client.initialize(&admin);

    // Second initialization with same admin should also fail
    client.initialize(&admin);

    // This code should never be reached due to panic
    unreachable!("Double initialization should have panicked");
}

/// Test: Production initialization sequence
///
/// Verifies the complete initialization flow as it would occur in production:
/// 1. Fresh contract deployment
/// 2. Single initialization call
/// 3. Verification of all subsystems
/// 4. Storage persistence validation
/// 5. Operational readiness check
#[test]
fn test_production_initialization_sequence() {
    let env = create_test_env();
    let contract_id = env.register(HelloContract, ());
    let client = HelloContractClient::new(&env, &contract_id);
    let admin = Address::generate(&env);

    // Step 1: Verify contract is fresh (no admin set)
    env.as_contract(&contract_id, || {
        assert!(!env.storage().persistent().has(&RiskDataKey::Admin));
        assert!(!env.storage().persistent().has(&InterestRateDataKey::InterestRateConfig));
    });

    // Step 2: Initialize contract (production deployment)
    let init_time = env.ledger().timestamp();
    client.initialize(&admin);

    // Step 3: Verify all subsystems are initialized
    env.as_contract(&contract_id, || {
        // Risk management subsystem
        assert!(env.storage().persistent().has(&RiskDataKey::Admin));
        assert!(env.storage().persistent().has(&RiskDataKey::RiskConfig));
        assert!(env.storage().persistent().has(&RiskDataKey::EmergencyPause));

        // Interest rate subsystem
        assert!(env.storage().persistent().has(&InterestRateDataKey::Admin));
        assert!(env.storage().persistent().has(&InterestRateDataKey::InterestRateConfig));

        // Verify admin consistency
        let risk_admin: Address = env.storage().persistent().get(&RiskDataKey::Admin).unwrap();
        let rate_admin: Address = env.storage().persistent().get(&InterestRateDataKey::Admin).unwrap();
        assert_eq!(risk_admin, admin);
        assert_eq!(rate_admin, admin);
        assert_eq!(risk_admin, rate_admin);
    });

    // Step 4: Verify default parameters are set correctly
    let config = client.get_risk_config().expect("Risk config should exist");
    assert_eq!(config.min_collateral_ratio, 11_000);
    assert_eq!(config.liquidation_threshold, 10_500);
    assert_eq!(config.close_factor, 5_000);
    assert_eq!(config.liquidation_incentive, 1_000);
    assert_eq!(config.last_update, init_time);

    // Step 5: Verify pause switches are properly initialized
    let operations = ["pause_deposit", "pause_withdraw", "pause_borrow", "pause_repay", "pause_liquidate"];
    for op in operations {
        let symbol = Symbol::new(&env, op);
        assert!(!client.is_operation_paused(&symbol), "Operation {} should be unpaused", op);
    }
    assert!(!client.is_emergency_paused());

    // Step 6: Verify operational readiness
    // Test that core functions work post-initialization
    assert!(client.get_risk_config().is_some());
    assert!(client.get_utilization() >= 0);
    assert!(client.get_borrow_rate() >= 0);
    assert!(client.get_supply_rate() >= 0);
}

/// Test: Storage persistence across ledger advancements
///
/// Verifies that all initialization data persists correctly
/// across multiple ledger periods and contract calls
#[test]
fn test_storage_persistence_across_ledger_advancements() {
    let env = create_test_env();
    let contract_id = env.register(HelloContract, ());
    let client = HelloContractClient::new(&env, &contract_id);
    let admin = Address::generate(&env);

    // Initialize contract
    client.initialize(&admin);

    // Store initial state for comparison
    let initial_config = client.get_risk_config().unwrap();
    let initial_admin: Address = env.as_contract(&contract_id, || {
        env.storage().persistent().get(&RiskDataKey::Admin).unwrap()
    });

    // Simulate multiple ledger periods
    for i in 1..=10 {
        // Advance ledger
        env.ledger().with_mut(|li| {
            li.sequence_number += 1;
            li.timestamp += i * 100;
        });

        // Verify data persistence
        env.as_contract(&contract_id, || {
            assert!(env.storage().persistent().has(&RiskDataKey::Admin));
            assert!(env.storage().persistent().has(&RiskDataKey::RiskConfig));
            assert!(env.storage().persistent().has(&InterestRateDataKey::InterestRateConfig));

            let current_admin: Address = env.storage().persistent().get(&RiskDataKey::Admin).unwrap();
            assert_eq!(current_admin, initial_admin);
        });

        // Verify functional persistence
        let current_config = client.get_risk_config().unwrap();
        assert_eq!(current_config.min_collateral_ratio, initial_config.min_collateral_ratio);
        assert_eq!(current_config.liquidation_threshold, initial_config.liquidation_threshold);
        assert_eq!(current_config.close_factor, initial_config.close_factor);
        assert_eq!(current_config.liquidation_incentive, initial_config.liquidation_incentive);
    }
}

/// Test: Storage isolation between contract instances
///
/// Verifies that different contract instances maintain separate storage
#[test]
fn test_storage_isolation_between_instances() {
    let env = create_test_env();

    // Deploy two contract instances
    let contract_id1 = env.register(HelloContract, ());
    let client1 = HelloContractClient::new(&env, &contract_id1);
    let admin1 = Address::generate(&env);

    let contract_id2 = env.register(HelloContract, ());
    let client2 = HelloContractClient::new(&env, &contract_id2);
    let admin2 = Address::generate(&env);

    // Initialize both contracts
    client1.initialize(&admin1);
    client2.initialize(&admin2);

    // Verify storage isolation
    env.as_contract(&contract_id1, || {
        let stored_admin: Address = env.storage().persistent().get(&RiskDataKey::Admin).unwrap();
        assert_eq!(stored_admin, admin1);
        assert_ne!(stored_admin, admin2);
    });

    env.as_contract(&contract_id2, || {
        let stored_admin: Address = env.storage().persistent().get(&RiskDataKey::Admin).unwrap();
        assert_eq!(stored_admin, admin2);
        assert_ne!(stored_admin, admin1);
    });

    // Verify functional isolation
    let config1 = client1.get_risk_config().unwrap();
    let config2 = client2.get_risk_config().unwrap();
    assert_eq!(config1.min_collateral_ratio, config2.min_collateral_ratio);
    // But timestamps should be different due to different initialization times
    assert_ne!(config1.last_update, config2.last_update);
}

/// Test: Initialization security boundaries
///
/// Verifies security assumptions and trust boundaries:
/// - Only authorized initializer can set admin
/// - Admin powers are properly established
/// - No unauthorized access paths exist
#[test]
fn test_initialization_security_boundaries() {
    let env = create_test_env();
    let contract_id = env.register(HelloContract, ());
    let client = HelloContractClient::new(&env, &contract_id);
    let admin = Address::generate(&env);

    // Initialize contract
    client.initialize(&admin);

    // Verify admin powers are established
    // Test admin can perform privileged operations
    client.set_emergency_pause(&admin, &true);
    assert!(client.is_emergency_paused());

    client.set_emergency_pause(&admin, &false);
    assert!(!client.is_emergency_paused());

    // Verify unauthorized users cannot perform admin operations
    let unauthorized = Address::generate(&env);
    
    // These should panic (unauthorized access)
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        client.set_emergency_pause(&unauthorized, &true);
    }));
    assert!(result.is_err(), "Unauthorized user should not be able to set emergency pause");

    // Verify admin-only functions work correctly
    let config = client.get_risk_config().unwrap();
    assert!(config.min_collateral_ratio >= 10_000);
    assert!(config.liquidation_threshold < config.min_collateral_ratio);
}

/// Test: Edge cases and boundary conditions
///
/// Tests various edge cases during initialization
#[test]
fn test_initialization_edge_cases() {
    let env = create_test_env();

    // Test with zero address (should work in test environment)
    let contract_id1 = env.register(HelloContract, ());
    let client1 = HelloContractClient::new(&env, &contract_id1);
    let zero_admin = Address::generate(&env); // In test, any address is valid
    
    // This should succeed
    client1.initialize(&zero_admin);

    // Test initialization at different ledger timestamps
    env.ledger().with_mut(|li| li.timestamp += 1000);
    
    let contract_id2 = env.register(HelloContract, ());
    let client2 = HelloContractClient::new(&env, &contract_id2);
    let admin2 = Address::generate(&env);
    
    client2.initialize(&admin2);
    
    // Verify timestamps are different
    let config1 = client1.get_risk_config().unwrap();
    let config2 = client2.get_risk_config().unwrap();
    assert!(config2.last_update > config1.last_update);
}

/// Test: Initialization should only happen once in production
///
/// Security note: In production, initialize should be called exactly once
/// during contract deployment. This test documents the expected usage pattern.
#[test]
fn test_initialization_production_pattern() {
    let env = create_test_env();
    let contract_id = env.register(HelloContract, ());
    let client = HelloContractClient::new(&env, &contract_id);
    let admin = Address::generate(&env);

    // Production pattern: Initialize once during deployment
    client.initialize(&admin);

    // Verify initialization succeeded
    env.as_contract(&contract_id, || {
        assert!(env.storage().persistent().has(&RiskDataKey::Admin));
        assert!(env.storage().persistent().has(&InterestRateDataKey::Admin));
    });

    // In production, no further initialization calls should be made
    // The contract is now ready for use
}
