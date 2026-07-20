//! Example: Run HSIP formal verification
//!
//! This example demonstrates how to use the HSIP formal verification system
//! to prove critical security properties using Z3 SMT solver.
//!
//! Run with: cargo run --example verify_hsip --features verification

use hsip_verify::{VerificationConfig, Verifier};

fn main() {
    println!("╔═══════════════════════════════════════════════════════════╗");
    println!("║      HSIP Formal Verification Example                    ║");
    println!("║      Using Z3 SMT Solver for Security Proofs             ║");
    println!("╚═══════════════════════════════════════════════════════════╝");
    println!();

    // Configure verification
    let config = VerificationConfig {
        timeout_ms: 10000, // 10 seconds per property
        generate_counterexamples: true,
        verbosity: 2, // Verbose output
    };

    println!("Configuration:");
    println!("  • Timeout: {} ms per property", config.timeout_ms);
    println!(
        "  • Counterexample generation: {}",
        config.generate_counterexamples
    );
    println!("  • Verbosity level: {}", config.verbosity);
    println!();

    // Create verifier and run all checks
    let verifier = Verifier::new(config);
    let report = verifier.verify_all();

    // Display results
    println!("\n{}", report);

    // Exit with appropriate code
    if report.all_proven() {
        println!("🎉 Success! All security properties have been formally verified.");
        std::process::exit(0);
    } else if report.has_violations() {
        eprintln!("💥 CRITICAL: Some security properties were violated!");
        std::process::exit(1);
    } else {
        eprintln!("⚠️  Warning: Some properties could not be verified (inconclusive).");
        std::process::exit(2);
    }
}
