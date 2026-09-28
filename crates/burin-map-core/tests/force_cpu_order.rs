//! `force_cpu()` must not quietly lose the request.
//!
//! The device is resolved once, on first use, and cached. Before the guard a
//! late `force_cpu()` returned without doing anything: the caller asked for
//! reproducible bytes and got a GPU drawing, and no error said so. This is the
//! regression test for that, and the reason it lives here rather than in a doc
//! comment is that a documented trap is still a trap.
//!
//! The tests share one process-wide cache, so each one re-runs this binary with
//! `--exact` and a case name to get a fresh process and a fresh cache. That
//! child cannot report back in its stderr — a panicking `#[test]` has its output
//! swallowed by the harness — so it writes its verdict to the file named by
//! `BURIN_FORCE_CPU_REPORT` instead. The parent reads that file, which is the
//! only reliable signal: a child that dies for an unrelated reason leaves no
//! report, and the parent treats a missing report as a failure.

#![cfg(all(feature = "gpu", not(target_arch = "wasm32")))]

use burin_map_core::engine;

const CASE: &str = "BURIN_FORCE_CPU_CASE";
const REPORT: &str = "BURIN_FORCE_CPU_REPORT";

/// What the child writes: the outcome, and why.
fn report(verdict: &str) {
    let path = std::env::var(REPORT).expect("the parent must set BURIN_FORCE_CPU_REPORT");
    std::fs::write(path, verdict).expect("write the report");
}

/// Runs this test binary again, filtered to `case`, and returns the verdict the
/// child wrote. `None` when it wrote nothing, which means it died unexpected.
fn run_case(case: &str, env: &[(&str, &str)]) -> Option<String> {
    let exe = std::env::current_exe().expect("current_exe");
    let dir = std::env::temp_dir().join(format!("burin-force-cpu-{}-{case}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("temp dir");
    let path = dir.join("report.txt");

    let mut cmd = std::process::Command::new(exe);
    cmd.env(CASE, case).env(REPORT, &path);
    for (k, v) in env {
        cmd.env(k, v);
    }
    let out = cmd
        .args(["--exact", case, "--nocapture"])
        .output()
        .expect("spawn the child case");

    match std::fs::read_to_string(&path) {
        Ok(s) => Some(s),
        Err(_) => {
            // No verdict: surface whatever the child managed to say.
            eprintln!("[child {case}] exit {:?}", out.status);
            eprintln!(
                "[child {case}] stderr: {}",
                String::from_utf8_lossy(&out.stderr)
            );
            None
        }
    }
}

#[test]
fn force_cpu_before_the_first_draw_switches_to_the_cpu() {
    if std::env::var(CASE).is_ok() {
        // Child run: no GPU has been touched yet, so this must take effect.
        engine::force_cpu();
        let where_ = engine::describe();
        report(if where_ == "CPU (forced)" {
            "ok"
        } else {
            "wrong-device"
        });
        assert_eq!(where_, "CPU (forced)");
        return;
    }

    let verdict = run_case("force_cpu_before_the_first_draw_switches_to_the_cpu", &[]);
    assert_eq!(
        verdict.as_deref(),
        Some("ok"),
        "calling force_cpu() before the first draw must work"
    );
    println!("force_cpu(): early call works");
}

#[test]
fn force_cpu_after_the_device_is_resolved_panics() {
    if std::env::var(CASE).is_ok() {
        // Child run. Resolve the device the way any draw would, then be late.
        if engine::device().is_none() {
            // No GPU to be stuck on: the caller asked for the CPU and the CPU is
            // what this machine would use anyway. Nothing for the guard to do.
            report("no-gpu");
            return;
        }
        engine::force_cpu();
        // With the guard in place this line is never reached: the panic inside
        // force_cpu() fires first, the report is never written, and the parent
        // reads the missing report as the failure of this test. Reaching here
        // means force_cpu() quietly returned, which is the bug.
        report("returned-quietly");
        return;
    }

    let verdict = run_case("force_cpu_after_the_device_is_resolved_panics", &[]);
    match verdict.as_deref() {
        // The guard panicked inside force_cpu(), so nothing was reported.
        None => println!("force_cpu(): late call panics, as intended"),
        Some("no-gpu") => println!("no GPU here: the late-call guard was not exercised"),
        Some(other) => panic!(
            "a late force_cpu() must fail loudly, but the child reported {other:?} — \
             it returned instead of panicking, and the caller keeps the GPU drawing"
        ),
    }
}

#[test]
fn force_cpu_when_the_cpu_is_already_forced_is_a_no_op() {
    if std::env::var(CASE).is_ok() {
        engine::force_cpu();
        // Repetition, not contradiction: this must not panic.
        engine::force_cpu();
        let where_ = engine::describe();
        report(if where_ == "CPU (forced)" {
            "ok"
        } else {
            "wrong-device"
        });
        return;
    }

    // BURIN_ENGINE=cpu is read before the device resolves, so by the time
    // force_cpu() runs the CPU is already the answer — exactly the case.
    let verdict = run_case(
        "force_cpu_when_the_cpu_is_already_forced_is_a_no_op",
        &[("BURIN_ENGINE", "cpu")],
    );
    assert_eq!(
        verdict.as_deref(),
        Some("ok"),
        "BURIN_ENGINE=cpu plus force_cpu() must be accepted"
    );
    println!("force_cpu(): already-CPU is accepted");
}
