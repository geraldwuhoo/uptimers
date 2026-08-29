use thiserror::Error;

#[derive(Error, Debug)]
pub enum UptimersError {
    #[error("IO error\n{0}")]
    Read(#[from] std::io::Error),

    #[error("askama templating error\n{0}")]
    Askama(#[from] askama::Error),

    #[error("reqwest error\n{0}")]
    Parse(#[from] reqwest::Error),

    #[error("sqlx error\n{0}")]
    Sqlx(#[from] sqlx::Error),

    #[error("sqlx migreate error\n{0}")]
    SqlxMigrate(#[from] sqlx::migrate::MigrateError),

    #[error("serde_yaml error\n{0}")]
    SerdeYaml(#[from] serde_yaml::Error),

    #[error("nul error \n{0}")]
    Nul(#[from] std::ffi::NulError),

    #[error("blocking task error \n{0}")]
    Join(#[from] actix_web::rt::task::JoinError),

    #[error("shoutrrr error \n{0}")]
    Shoutrrr(String),
}

impl actix_web::error::ResponseError for UptimersError {}
