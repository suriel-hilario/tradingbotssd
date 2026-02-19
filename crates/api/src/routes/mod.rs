mod api;
mod health;
mod static_files;
mod ws;

pub use api::api_router;
pub use health::health_router;
pub use static_files::static_router;
pub use ws::ws_router;
