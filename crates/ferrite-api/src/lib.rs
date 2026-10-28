pub const API_VERSION: &str = env!("CARGO_PKG_VERSION");

pub mod prelude {
    pub use crate::API_VERSION;
}
