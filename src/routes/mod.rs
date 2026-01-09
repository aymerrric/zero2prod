pub mod change_password;
pub mod dashboard;
pub mod health_check;
pub mod home;
pub mod log_out;
pub mod login;
pub mod newsletter;
pub mod subscriptions;
pub mod subscriptions_confirm;

pub use change_password::{form_password, password_change};
pub use dashboard::admin_dashboard;
pub use health_check::*;
pub use home::*;
pub use log_out::*;
pub use login::*;
pub use newsletter::*;
pub use subscriptions::*;
pub use subscriptions_confirm::*;
