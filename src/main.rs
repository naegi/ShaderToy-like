mod app;
mod camera;

fn main() {
    env_logger::init_from_env(
        env_logger::Env::default().filter_or(env_logger::DEFAULT_FILTER_ENV, "info"),
    );
    let mut app = app::App::new_block();
    app.run();
}
