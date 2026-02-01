# Phase 2: Quick Start Guide

**Read this first before starting Phase 2 implementation.**

---

## 📚 **Reading Order**

### **Step 1: Understand the Architecture**
1. **[../PHASE_2_ARCHITECTURE.md](../PHASE_2_ARCHITECTURE.md)** ⭐ START HERE
   - All architecture decisions
   - Design patterns
   - Trade-offs explained
   - ~30 min read

### **Step 2: Review the Roadmap**
2. **[../../ROADMAP.md](../../ROADMAP.md)** - Phase 2 section
   - Timeline
   - Task breakdown
   - Deliverables
   - ~15 min read

### **Step 3: Implementation Tasks** (Read as needed)
Read these in order as you implement:

| Week | Task | File | Time | Priority |
|------|------|------|------|----------|
| **Week 1** | Foundation | [phase2-foundation.md](phase2-foundation.md) | 5-7 days | ⭐ Critical |
| **Week 2** | Protocol | [phase2-network-protocol.md](phase2-network-protocol.md) | 3-4 days | ⭐ Critical |
| **Week 2** | TCP | [phase2-tcp-connection.md](phase2-tcp-connection.md) | 4-5 days | ⭐ Critical |
| **Week 3** | UDP | [phase2-udp-packets.md](phase2-udp-packets.md) | 3-4 days | High |
| **Week 3** | State Sync | [phase2-state-sync.md](phase2-state-sync.md) | 5-7 days | ⭐ Critical |
| **Week 4** | Prediction | [phase2-client-prediction.md](phase2-client-prediction.md) | 5-6 days | High |
| **Week 4** | Server Tick | [phase2-server-tick.md](phase2-server-tick.md) | 4-5 days | ⭐ Critical |
| **Week 4** | Interest Mgmt | [phase2-interest-basic.md](phase2-interest-basic.md) | 3-4 days | Medium |

---

## 🎯 **Key Decisions Summary**

### **1. Feature Flags**
```rust
#[client_only]              // Only client
#[server_only]              // Only server
#[shared]                   // Both
#[server_authoritative]     // Different implementations
```

### **2. Testing**
- Comprehensive (like Phase 1.4)
- >80% coverage
- Property tests for all network code
- Benchmarks for performance

### **3. Metrics**
- Built-in Prometheus endpoint
- Optional (can disable)
- Track everything (TPS, latency, errors, etc.)

### **4. Containers**
- Docker from day one
- `./scripts/dev.sh` starts environment
- Production images <50MB

### **5. Protocol**
- FlatBuffers (zero-copy)
- TCP + UDP dual channel
- Versioned (semantic versioning)
- Length-prefix framing

### **6. State Sync**
- Adaptive full + delta
- Property tests verify correctness
- 80-90% bandwidth reduction

### **7. Prediction**
- Client predicts + server reconciles
- Target: <10% error rate
- Replay unconfirmed inputs

---

## 🚀 **Implementation Checklist**

### **Before You Start**
- [ ] Read PHASE_2_ARCHITECTURE.md
- [ ] Review Phase 1.4 completion (macros, tests, benchmarks)
- [ ] Ensure Docker Desktop installed
- [ ] Ensure Rust 1.75+ installed

### **Week 1: Foundation**
- [ ] Read phase2-foundation.md
- [ ] Implement proc macros
- [ ] Setup build infrastructure
- [ ] Create Docker environment
- [ ] Implement metrics endpoint
- [ ] Create admin console
- [ ] Verify: `./scripts/dev.sh` works

### **Week 2: Protocol & TCP**
- [ ] Read phase2-network-protocol.md
- [ ] Define FlatBuffers schema
- [ ] Implement message serialization
- [ ] Read phase2-tcp-connection.md
- [ ] Implement TCP server
- [ ] Implement TCP client
- [ ] Verify: Client connects to server

### **Week 3: UDP & State Sync**
- [ ] Read phase2-udp-packets.md
- [ ] Implement UDP channel
- [ ] Read phase2-state-sync.md
- [ ] Implement full state sync
- [ ] Implement delta compression
- [ ] Verify: World state syncs

### **Week 4: Prediction & Server**
- [ ] Read phase2-client-prediction.md
- [ ] Implement client prediction
- [ ] Implement reconciliation
- [ ] Read phase2-server-tick.md
- [ ] Implement server tick loop
- [ ] Read phase2-interest-basic.md
- [ ] Implement basic culling
- [ ] Verify: Multiplayer demo works

### **Week 5: Polish**
- [ ] Complete all tests
- [ ] Run all benchmarks
- [ ] Fix any issues
- [ ] Complete documentation
- [ ] Verify: All acceptance criteria met

---

## 📊 **Performance Targets**

Copy these into your test suite:

```rust
// Network
assert!(serialize_time < Duration::from_micros(1));
assert!(deserialize_time < Duration::from_nanos(500));
assert!(tcp_send_time < Duration::from_micros(100));
assert!(udp_send_time < Duration::from_micros(50));

// Server
assert!(tick_duration < Duration::from_millis(16));
assert!(physics_time < Duration::from_millis(8));
assert!(network_time < Duration::from_millis(3));

// Client
assert!(frame_time < Duration::from_micros(16670));
assert!(prediction_time < Duration::from_micros(500));

// Bandwidth (per client, 60 TPS)
assert!(outgoing_bytes_per_sec < 5_000);    // 5 KB/s
assert!(incoming_bytes_per_sec < 20_000);   // 20 KB/s
```

---

## 🧪 **Testing Checklist**

For each subsystem:

```rust
// 1. Unit tests
#[test]
fn test_basic_functionality() { }

// 2. Property tests
proptest! {
    #[test]
    fn test_invariants(/* random inputs */) { }
}

// 3. Integration tests
#[tokio::test]
async fn test_client_server_interaction() { }

// 4. Benchmarks
fn bench_performance(c: &mut Criterion) { }

// 5. Compile-fail tests (for macros)
#[test]
fn test_separation_enforced() {
    let t = trybuild::TestCases::new();
    t.compile_fail("tests/compile_fail/*.rs");
}
```

---

## 🐛 **Common Issues & Solutions**

### **Issue: Docker not starting**
```bash
# Check Docker is running
docker info

# If not, start Docker Desktop
```

### **Issue: Hot-reload not working**
```bash
# Rebuild containers
docker-compose -f docker-compose.dev.yml build

# Restart
docker-compose -f docker-compose.dev.yml down
docker-compose -f docker-compose.dev.yml up
```

### **Issue: Metrics not accessible**
```bash
# Check server is running
curl http://localhost:8080/health

# Check Prometheus config
cat monitoring/prometheus.yml

# View server logs
docker-compose -f docker-compose.dev.yml logs server
```

### **Issue: Property tests failing**
```rust
// Reduce test cases for debugging
proptest! {
    #![proptest_config(ProptestConfig::with_cases(10))]
    #[test]
    fn test_something(/* inputs */) { }
}

// Add logging
println!("Testing with input: {:?}", input);
```

### **Issue: Benchmarks too slow**
```rust
// Reduce sample size
c.bench_function("test", |b| {
    b.iter_batched(
        || setup(),
        |input| test(input),
        BatchSize::SmallInput  // or LargeInput
    );
});
```

---

## 💡 **Pro Tips**

### **Development**
1. **Start simple:** Get basic functionality working first, optimize later
2. **Test early:** Write tests as you implement, not after
3. **Measure everything:** Use metrics to guide optimization
4. **Read logs:** Docker logs are your friend

### **Debugging**
1. **Use admin console:** Quick way to inspect server state
2. **Check metrics:** Performance issues show up in metrics
3. **Property tests:** Find edge cases you wouldn't think of
4. **Wireshark:** Capture network traffic if something strange

### **Performance**
1. **Profile first:** Don't optimize without measuring
2. **Benchmark often:** Catch regressions early
3. **Property tests catch bugs:** That benchmarks might miss
4. **Zero-copy:** Use FlatBuffers correctly for maximum benefit

---

## 📞 **Getting Help**

If stuck:

1. **Check docs:** Re-read the architecture document
2. **Check tests:** Look at existing tests for examples
3. **Check metrics:** See if something is failing
4. **Check logs:** Docker logs often reveal issues
5. **Ask questions:** Document unclear? Note it for improvement

---

## ✅ **Definition of Done**

A task is complete when:

- [ ] All code implemented
- [ ] All tests passing (unit + property + integration + benchmark)
- [ ] All benchmarks meeting targets
- [ ] CI green on all platforms
- [ ] Documentation complete
- [ ] Acceptance criteria met (from task file)
- [ ] Code reviewed (self-review checklist)
- [ ] No clippy warnings

---

## 🎯 **Phase 2 Success = Multiplayer Demo**

By end of Phase 2, you should be able to:

```bash
# Terminal 1: Start server
./scripts/dev.sh

# Terminal 2: Start client 1
cargo run --bin client

# Terminal 3: Start client 2
cargo run --bin client

# Both clients should:
# - Connect to server
# - See each other
# - Move around smoothly
# - <50ms latency
# - <10% prediction errors
```

If this works, Phase 2 is successful! 🎉

---

**Last Updated:** 2026-02-01
**Status:** Ready for Implementation
**Good luck!** 🚀
