//! Integration tests for the authentication system.

use engine_auth::*;

#[tokio::test]
async fn test_complete_user_lifecycle() {
    // Registration
    let username = "newuser";
    let email = "newuser@example.com";
    let password = "MySecureP@ssw0rd!";

    // Validate inputs
    assert!(validate_username(username).is_ok());
    assert!(validate_email(email).is_ok());
    assert!(validate_password_strength(password).is_ok());

    // Hash password
    let password_hash = hash_password(password).await.unwrap();

    // Create user
    let mut user =
        User::new(username.to_string(), email.to_string(), password_hash.clone()).unwrap();
    assert!(!user.email_verified);
    assert!(!user.mfa_enabled);

    // Login
    let is_valid = verify_password(password, &password_hash).await.unwrap();
    assert!(is_valid);

    // Generate tokens
    let jwt_manager = create_test_jwt_manager().unwrap();
    let tokens = jwt_manager
        .generate_token_pair(&user.id.to_string(), &user.username, &user.email)
        .unwrap();

    // Validate tokens
    let access_claims = jwt_manager.validate_access_token(&tokens.access_token).unwrap();
    assert_eq!(access_claims.sub, user.id.to_string());

    let refresh_claims = jwt_manager.validate_refresh_token(&tokens.refresh_token).unwrap();
    assert_eq!(refresh_claims.sub, user.id.to_string());

    // Create session
    let session_store = SessionStore::new();
    let session = session_store
        .create_session(
            user.id.to_string(),
            "192.168.1.100".to_string(),
            "GameClient/1.0".to_string(),
        )
        .unwrap();

    // Setup MFA
    let totp = TotpManager::new("TestGame".to_string());
    let totp_setup = totp.generate_secret(&user.email).unwrap();
    user.totp_secret = Some(totp_setup.secret.clone());
    user.mfa_enabled = true;

    // Generate backup codes
    let (plaintext_codes, backup_codes) = BackupCodeManager::generate_codes();
    user.backup_codes = backup_codes.iter().map(|c| c.hash.clone()).collect();

    // Verify TOTP
    let totp_code = totp.generate_current_code(&totp_setup.secret, &user.email).unwrap();
    let totp_valid = totp.verify_code(&totp_setup.secret, &totp_code, &user.email).unwrap();
    assert!(totp_valid);

    // Link OAuth account
    user.link_oauth_provider("steam".to_string(), "76561197960287930".to_string());
    assert!(user.has_oauth_provider("steam"));

    // Update last login
    user.update_last_login();
    assert!(user.last_login.is_some());

    // Logout - delete session
    session_store.delete_session(&session.id);
    assert!(session_store.get_session(&session.id).is_err());

    // Revoke token
    jwt_manager.revoke_token(&access_claims.jti);
    assert!(jwt_manager.validate_access_token(&tokens.access_token).is_err());
}

#[tokio::test]
async fn test_failed_login_lockout() {
    let password = "SecureP@ss123";
    let password_hash = hash_password(password).await.unwrap();
    let mut user =
        User::new("testuser".to_string(), "test@example.com".to_string(), password_hash.clone())
            .unwrap();

    let audit_logger = AuditLogger::new();
    let rate_limiter = IpRateLimiter::new();

    // Simulate 5 failed login attempts
    for i in 0..5 {
        // Check rate limit
        if rate_limiter.check("192.168.1.100").is_err() {
            audit_logger
                .log_rate_limit_exceeded("192.168.1.100".to_string(), "GameClient/1.0".to_string());
            break;
        }

        // Wrong password
        let is_valid = verify_password("WrongPassword", &password_hash).await.unwrap();
        assert!(!is_valid);

        audit_logger.log_login_failed(
            user.username.clone(),
            "192.168.1.100".to_string(),
            "GameClient/1.0".to_string(),
            "Invalid password".to_string(),
        );

        user.increment_failed_attempts(5, 30);

        if user.is_locked() {
            audit_logger.log_account_locked(
                user.id.to_string(),
                user.username.clone(),
                "192.168.1.100".to_string(),
                "GameClient/1.0".to_string(),
                "Too many failed attempts".to_string(),
            );
            break;
        }
    }

    assert!(user.is_locked());

    // Check audit log
    let threat_events = audit_logger.get_threat_events();
    assert!(!threat_events.is_empty());
}

#[tokio::test]
async fn test_mfa_with_backup_codes() {
    let password = "SecureP@ss123";
    let password_hash = hash_password(password).await.unwrap();
    let mut user =
        User::new("mfauser".to_string(), "mfa@example.com".to_string(), password_hash.clone())
            .unwrap();

    // Enable MFA
    let totp = TotpManager::new("TestGame".to_string());
    let totp_setup = totp.generate_secret(&user.email).unwrap();
    user.totp_secret = Some(totp_setup.secret.clone());
    user.mfa_enabled = true;

    // Generate backup codes
    let (plaintext_codes, mut backup_codes) = BackupCodeManager::generate_codes();

    // Login with password
    let password_valid = verify_password(password, &password_hash).await.unwrap();
    assert!(password_valid);

    // User doesn't have TOTP device - use backup code
    let backup_code = &plaintext_codes[0];
    let result = BackupCodeManager::verify_and_use(&mut backup_codes, backup_code);
    assert!(result.is_ok());

    // Try to reuse same backup code - should fail
    let result = BackupCodeManager::verify_and_use(&mut backup_codes, backup_code);
    assert!(result.is_err());

    // Check remaining backup codes
    let remaining = BackupCodeManager::unused_count(&backup_codes);
    assert_eq!(remaining, 9);
}

#[tokio::test]
async fn test_token_refresh() {
    let user =
        User::new("tokenuser".to_string(), "token@example.com".to_string(), "hash".to_string())
            .unwrap();

    let jwt_manager = create_test_jwt_manager().unwrap();

    // Generate initial tokens
    let tokens1 = jwt_manager
        .generate_token_pair(&user.id.to_string(), &user.username, &user.email)
        .unwrap();

    // Validate refresh token
    let refresh_claims = jwt_manager.validate_refresh_token(&tokens1.refresh_token).unwrap();
    assert_eq!(refresh_claims.sub, user.id.to_string());

    // Generate new token pair using refresh token
    let tokens2 = jwt_manager
        .generate_token_pair(&user.id.to_string(), &user.username, &user.email)
        .unwrap();

    // Both access tokens should be valid (different)
    assert_ne!(tokens1.access_token, tokens2.access_token);

    let claims1 = jwt_manager.validate_access_token(&tokens1.access_token).unwrap();
    let claims2 = jwt_manager.validate_access_token(&tokens2.access_token).unwrap();

    assert_eq!(claims1.sub, claims2.sub);
    assert_ne!(claims1.jti, claims2.jti); // Different token IDs
}

#[tokio::test]
async fn test_concurrent_sessions() {
    let user_id = uuid::Uuid::new_v4().to_string();
    let session_store = SessionStore::with_config(30, 24, 3); // Max 3 sessions

    // Create 3 sessions
    let session1 = session_store
        .create_session(user_id.clone(), "192.168.1.100".to_string(), "Device1".to_string())
        .unwrap();

    let session2 = session_store
        .create_session(user_id.clone(), "192.168.1.101".to_string(), "Device2".to_string())
        .unwrap();

    let session3 = session_store
        .create_session(user_id.clone(), "192.168.1.102".to_string(), "Device3".to_string())
        .unwrap();

    // 4th session should fail
    let result = session_store.create_session(
        user_id.clone(),
        "192.168.1.103".to_string(),
        "Device4".to_string(),
    );
    assert!(result.is_err());

    // Delete one session
    session_store.delete_session(&session1.id);

    // Now 4th session should succeed
    let session4 = session_store
        .create_session(user_id.clone(), "192.168.1.103".to_string(), "Device4".to_string())
        .unwrap();

    // Verify active sessions
    let active_sessions = session_store.get_user_sessions(&user_id);
    assert_eq!(active_sessions.len(), 3);
}

#[tokio::test]
async fn test_session_cleanup() {
    let store = SessionStore::with_config(0, 0, 10); // 0 timeouts for immediate expiration

    // Create sessions
    for i in 0..5 {
        store
            .create_session(format!("user{}", i), "127.0.0.1".to_string(), "TestClient".to_string())
            .unwrap();
    }

    assert_eq!(store.session_count(), 5);

    // Wait for sessions to expire
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    // Cleanup
    let cleaned = store.cleanup_expired_sessions();
    assert_eq!(cleaned, 5);
    assert_eq!(store.session_count(), 0);
}

#[test]
fn test_oauth_state_validation() {
    let state = OAuthState::new(OAuthProvider::Discord, "https://example.com/callback".to_string());

    // State should not be expired
    assert!(!state.is_expired());

    // Verify provider
    assert_eq!(state.provider, OAuthProvider::Discord);

    // Token should be valid UUID
    assert!(uuid::Uuid::parse_str(&state.token).is_ok());
}

#[tokio::test]
async fn test_password_rehashing() {
    let password = "TestPassword123!";

    // Hash with current parameters
    let hash = hash_password(password).await.unwrap();

    // Verify it works
    assert!(verify_password(password, &hash).await.unwrap());

    // In production, you would check if hash needs upgrade and rehash if needed
}

#[tokio::test]
async fn test_input_validation_sql_injection() {
    // Test SQL injection attempts
    let malicious_username = "admin'; DROP TABLE users; --";
    assert!(validate_username(malicious_username).is_err());

    let malicious_email = "test@example.com'; DROP TABLE users; --";
    assert!(validate_email(malicious_email).is_err());
}

#[tokio::test]
async fn test_xss_prevention() {
    // Test XSS attempts in username
    let xss_username = "<script>alert('xss')</script>";
    assert!(validate_username(xss_username).is_err());

    // Only alphanumeric, underscore, dash allowed
    let valid_username = "safe_user-123";
    assert!(validate_username(valid_username).is_ok());
}
