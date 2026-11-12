use std::sync::LazyLock;

pub const VERSION: &str = env!("CARGO_PKG_VERSION");
pub const VERSION_MAJOR: &str = env!("CARGO_PKG_VERSION_MAJOR");
pub const VERSION_MINOR: &str = env!("CARGO_PKG_VERSION_MINOR");
pub const VERSION_PATCH: &str = env!("CARGO_PKG_VERSION_PATCH");

/// The numerical version calculated at the first access.
///
/// | Major | Minor |
/// | ----- | ----- |
/// | 3     | 20    |
///
/// -> 320
pub static VERSION_NUMBER: LazyLock<u32> = LazyLock::new(|| {
    let major: u32 = VERSION_MAJOR
        .parse()
        .expect("Failed to parse major version!");
    let minor: u32 = VERSION_MINOR
        .parse()
        .expect("Failed to parse minor version!");
    let places = minor.to_string().len() as u32;
    major * 10_u32.pow(places) + minor
});
