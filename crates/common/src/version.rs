use std::sync::LazyLock;

pub type VersionNumber = u32;

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
pub static VERSION_NUMBER: LazyLock<VersionNumber> = LazyLock::new(|| {
    let major: VersionNumber = VERSION_MAJOR
        .parse()
        .expect("Failed to parse major version!");
    let minor: VersionNumber = VERSION_MINOR
        .parse()
        .expect("Failed to parse minor version!");
    let places = minor.to_string().len() as u32;
    major * 10_u32.pow(places) + minor
});
