//! Authentication system benchmarks.

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use engine_auth::*;
use tokio::runtime::Runtime;

fn bench_password_hashing(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();

    c.bench_function("password_hash", |b| {
        b.to_async(&rt).iter(|| async {
            black_box(hash_password("TestPassword123!").await.unwrap());
        });
    });
}

fn bench_password_verification(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let hash = rt.block_on(hash_password("TestPassword123!")).unwrap();

    c.bench_function("password_verify", |b| {
        b.to_async(&rt).iter(|| async {
            black_box(verify_password("TestPassword123!", &hash).await.unwrap());
        });
    });
}

fn bench_jwt_generation(c: &mut Criterion) {
    let jwt_manager = create_test_jwt_manager().unwrap();

    c.bench_function("jwt_generate_token_pair", |b| {
        b.iter(|| {
            black_box(
                jwt_manager
                    .generate_token_pair("user123", "testuser", "test@example.com")
                    .unwrap(),
            );
        });
    });
}

fn bench_jwt_validation(c: &mut Criterion) {
    let jwt_manager = create_test_jwt_manager().unwrap();
    let tokens = jwt_manager
        .generate_token_pair("user123", "testuser", "test@example.com")
        .unwrap();

    c.bench_function("jwt_validate_access_token", |b| {
        b.iter(|| {
            black_box(jwt_manager.validate_access_token(&tokens.access_token).unwrap());
        });
    });
}

fn bench_session_operations(c: &mut Criterion) {
    let store = SessionStore::new();

    let mut group = c.benchmark_group("session");

    group.bench_function("session_create", |b| {
        b.iter(|| {
            black_box(
                store
                    .create_session(
                        uuid::Uuid::new_v4().to_string(),
                        "127.0.0.1".to_string(),
                        "TestClient".to_string(),
                    )
                    .unwrap(),
            );
        });
    });

    // Pre-create sessions for lookup benchmark
    let session = store
        .create_session("user123".to_string(), "127.0.0.1".to_string(), "TestClient".to_string())
        .unwrap();

    group.bench_function("session_get", |b| {
        b.iter(|| {
            black_box(store.get_session(&session.id).unwrap());
        });
    });

    group.finish();
}

fn bench_totp_operations(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let totp = TotpManager::new("TestApp".to_string());

    let mut group = c.benchmark_group("totp");

    group.bench_function("totp_generate_secret", |b| {
        b.iter(|| {
            black_box(totp.generate_secret("test@example.com").unwrap());
        });
    });

    let setup = totp.generate_secret("test@example.com").unwrap();
    let code = totp.generate_current_code(&setup.secret, "test@example.com").unwrap();

    group.bench_function("totp_verify_code", |b| {
        b.iter(|| {
            black_box(totp.verify_code(&setup.secret, &code, "test@example.com").unwrap());
        });
    });

    group.finish();
}

fn bench_backup_codes(c: &mut Criterion) {
    let mut group = c.benchmark_group("backup_codes");

    group.bench_function("backup_codes_generate", |b| {
        b.iter(|| {
            black_box(BackupCodeManager::generate_codes());
        });
    });

    let (plaintext, mut hashed) = BackupCodeManager::generate_codes();

    group.bench_function("backup_codes_verify", |b| {
        b.iter_batched(
            || (plaintext[0].clone(), hashed.clone()),
            |(code, mut codes)| {
                black_box(BackupCodeManager::verify_and_use(&mut codes, &code).unwrap());
            },
            criterion::BatchSize::SmallInput,
        );
    });

    group.finish();
}

fn bench_rate_limiting(c: &mut Criterion) {
    let limiter = RateLimiter::with_config(1000, 1); // High limit for benchmarking

    c.bench_function("rate_limiter_check", |b| {
        b.iter(|| {
            black_box(limiter.check("test").unwrap());
        });
    });
}

fn bench_audit_logging(c: &mut Criterion) {
    let logger = AuditLogger::new();

    c.bench_function("audit_log_event", |b| {
        b.iter(|| {
            logger.log_login_success(
                "user123".to_string(),
                "testuser".to_string(),
                "127.0.0.1".to_string(),
                "TestClient".to_string(),
            );
        });
    });
}

fn bench_user_validation(c: &mut Criterion) {
    let mut group = c.benchmark_group("validation");

    group.bench_function("validate_username", |b| {
        b.iter(|| {
            black_box(validate_username("testuser").unwrap());
        });
    });

    group.bench_function("validate_email", |b| {
        b.iter(|| {
            black_box(validate_email("test@example.com").unwrap());
        });
    });

    group.bench_function("validate_password_strength", |b| {
        b.iter(|| {
            black_box(validate_password_strength("StrongP@ss123").unwrap());
        });
    });

    group.finish();
}

fn bench_concurrent_logins(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();

    let mut group = c.benchmark_group("concurrent_operations");

    for &num_concurrent in &[10, 100, 1000] {
        group.throughput(Throughput::Elements(num_concurrent));
        group.bench_with_input(
            BenchmarkId::new("concurrent_logins", num_concurrent),
            &num_concurrent,
            |b, &num| {
                b.to_async(&rt).iter(|| async move {
                    let jwt_manager = create_test_jwt_manager().unwrap();
                    let session_store = SessionStore::new();

                    let mut handles = Vec::new();
                    for i in 0..num {
                        let jwt = jwt_manager.clone();
                        let store = session_store.clone();

                        let handle = tokio::spawn(async move {
                            // Simulate login
                            let password = "TestPassword123!";
                            let hash = hash_password(password).await.unwrap();
                            let _ = verify_password(password, &hash).await.unwrap();

                            // Generate tokens
                            let user_id = format!("user{}", i);
                            let tokens = jwt
                                .generate_token_pair(&user_id, &user_id, "test@example.com")
                                .unwrap();

                            // Create session
                            let _ = store
                                .create_session(
                                    user_id,
                                    "127.0.0.1".to_string(),
                                    "TestClient".to_string(),
                                )
                                .unwrap();

                            tokens
                        });
                        handles.push(handle);
                    }

                    for handle in handles {
                        let _ = handle.await;
                    }
                });
            },
        );
    }

    group.finish();
}

fn bench_session_lookup_scaling(c: &mut Criterion) {
    let mut group = c.benchmark_group("session_scaling");

    for &num_sessions in &[100, 1000, 10000] {
        group.throughput(Throughput::Elements(1));

        // Create store with many sessions
        let store = SessionStore::with_config(60, 24, 100000);
        let mut session_ids = Vec::new();

        for i in 0..num_sessions {
            let session = store
                .create_session(
                    format!("user{}", i),
                    "127.0.0.1".to_string(),
                    "TestClient".to_string(),
                )
                .unwrap();
            session_ids.push(session.id);
        }

        let lookup_id = session_ids[num_sessions / 2].clone();

        group.bench_with_input(
            BenchmarkId::new("session_lookup", num_sessions),
            &lookup_id,
            |b, id| {
                b.iter(|| {
                    black_box(store.get_session(id).unwrap());
                });
            },
        );
    }

    group.finish();
}

fn bench_login_throughput(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();

    let mut group = c.benchmark_group("login_throughput");
    group.throughput(Throughput::Elements(1));

    // Pre-compute password hash
    let password = "TestPassword123!";
    let hash = rt.block_on(hash_password(password)).unwrap();

    let jwt_manager = create_test_jwt_manager().unwrap();
    let session_store = SessionStore::new();
    let audit_logger = AuditLogger::new();

    group.bench_function("full_login_flow", |b| {
        b.to_async(&rt).iter(|| async {
            // Verify password
            let is_valid = verify_password(password, &hash).await.unwrap();
            assert!(is_valid);

            // Generate tokens
            let tokens = jwt_manager
                .generate_token_pair("user123", "testuser", "test@example.com")
                .unwrap();

            // Validate token
            let _ = jwt_manager.validate_access_token(&tokens.access_token).unwrap();

            // Create session
            let session = session_store
                .create_session(
                    "user123".to_string(),
                    "127.0.0.1".to_string(),
                    "TestClient".to_string(),
                )
                .unwrap();

            // Log event
            audit_logger.log_login_success(
                "user123".to_string(),
                "testuser".to_string(),
                "127.0.0.1".to_string(),
                "TestClient".to_string(),
            );

            black_box((tokens, session));
        });
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_password_hashing,
    bench_password_verification,
    bench_jwt_generation,
    bench_jwt_validation,
    bench_session_operations,
    bench_totp_operations,
    bench_backup_codes,
    bench_rate_limiting,
    bench_audit_logging,
    bench_user_validation,
    bench_concurrent_logins,
    bench_session_lookup_scaling,
    bench_login_throughput,
);

criterion_main!(benches);
