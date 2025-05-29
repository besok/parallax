use bevy::prelude::Component;

pub mod actors;
pub mod error;
pub mod vis2d;

pub fn init_logger() {
    use chrono::Local;
    use env_logger::{Builder, Env};
    use std::io::Write;

    Builder::from_env(Env::default().default_filter_or("info"))
        .format(|buf, record| {
            let timestamp = Local::now().format("%H:%M:%S%.3f"); // Include milliseconds

            writeln!(
                buf,
                "[{} {} {}] {}",
                timestamp,
                record.level().to_string().chars().next().unwrap_or('I'),
                record
                    .target()
                    .split("::")
                    .last()
                    .unwrap_or(record.target()),
                record.args()
            )
        })
        .init();
}
type Res<T> = Result<T, error::KernelError>;
type VoidRes = Res<()>;
