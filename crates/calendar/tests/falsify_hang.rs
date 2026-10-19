//! FALSIFICATION ONLY, reverted before this pull request leaves draft (lane
//! ci/hung-shard-leaves-evidence). A host test that hangs, in CI's `test-shard` job alone, with a
//! paused QEMU as its child, so the run shows what `helpers/hang_evidence.py` leaves.

#[test]
fn a_hung_host_test_with_a_qemu_child() {
    if std::env::var("GITHUB_JOB").as_deref() != Ok("test-shard") {
        return;
    }
    let mut qemu = std::process::Command::new("qemu-system-aarch64")
        .args(["-M", "virt", "-cpu", "cortex-a57", "-S", "-monitor", "none"])
        .args(["-serial", "null", "-display", "none"])
        .spawn()
        .expect("qemu-system-aarch64 on PATH");
    let _ = qemu.wait();
    loop {
        std::thread::park();
    }
}
