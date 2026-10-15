//! **Opening, naming and handles**: a file the host tool wrote, names that do not walk, and handles
//! that are forged, closed or reused.
//!
//! Name: provisional (lane maintainer/split-parents-to-zero, 2026-10-13 UTC).

use super::*;

#[test]
fn opens_and_reads_a_file_the_host_tool_wrote() {
    let mut srv = server_with(&[("motd", b"hello from redoxfs\n")]);
    let h = srv.open_file(&named("motd")).expect("open motd");
    let mut buf = [0u8; 64];
    let n = srv.read(h, 0, &mut buf).expect("read");
    assert_eq!(&buf[..n], b"hello from redoxfs\n");
}

#[test]
fn a_missing_name_is_enoent_and_names_do_not_walk() {
    let mut srv = server_with(&[("present", b"x")]);
    assert_eq!(
        srv.open_file(&named("absent")).unwrap_err().errno,
        syscall::error::ENOENT
    );
    // A name is one component resolved in the bound directory, never a path: a slash does not
    // descend, so "present/nope" finds nothing rather than walking anywhere.
    assert!(srv.open_file(&named("present/nope")).is_err());
}

#[test]
fn a_forged_or_closed_handle_is_ebadf() {
    let mut srv = server_with(&[("f", b"data")]);
    let h = srv.open_file(&named("f")).unwrap();
    let mut buf = [0u8; 8];
    assert_eq!(srv.read(999, 0, &mut buf).unwrap_err().errno, EBADF);
    srv.close(h).unwrap();
    assert_eq!(srv.read(h, 0, &mut buf).unwrap_err().errno, EBADF);
    assert_eq!(srv.close(h).unwrap_err().errno, EBADF);
}

#[test]
fn handles_are_reused_after_close() {
    let mut srv = server_with(&[("a", b"1"), ("b", b"2")]);
    let h0 = srv.open_file(&named("a")).unwrap();
    assert_ne!(
        h0, 0,
        "slot 0 is the bound directory and is never handed out"
    );
    srv.close(h0).unwrap();
    let h1 = srv.open_file(&named("b")).unwrap();
    assert_eq!(h0, h1, "a freed slot is reused before growing the table");
}
