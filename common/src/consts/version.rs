/// Defines the protocol version constants. Update the version string here only.
macro_rules! define_protocol_version {
    ($version:literal) => {
        pub const PROTOCOL_VERSION: &str = $version;
        /// Null-terminated version for FFI use
        pub const PROTOCOL_VERSION_CSTR: &[u8] = concat!($version, "\0").as_bytes();
    };
}

define_protocol_version!("1.3.0");
