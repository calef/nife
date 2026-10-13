/// `CTRL.GIO_MASTER_DISABLE`: stop new bus-master requests before the reset.
pub const CTRL_GIO_MASTER_DISABLE: u32 = 0x0000_0004;
/// `STATUS.GIO_MASTER_ENABLE`: set while master requests are still pending.
pub const STATUS_GIO_MASTER_ENABLE: u32 = 0x0008_0000;
/// How long to wait for pending master requests, in microseconds: 800 polls of 100.
pub const MASTER_WAIT_US: u64 = 80_000;
/// `KABGTXD.BGSQLBIAS`, set after the reset.
pub const KABGTXD_BGSQLBIAS: u32 = 0x0005_0000;
