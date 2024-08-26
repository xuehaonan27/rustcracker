use std::env;

mod demo;

#[tokio::main]
async fn main() {
    log4rs::init_file("config/log4rs.yaml", Default::default()).unwrap();

    let set = env::args()
        .skip(1)
        .next()
        .expect("Must specify with demo to run");

    let _: () = match set.as_str() {
        "--no-jailer" => demo::no_jailer().await,
        "--with-jailer" => demo::with_jailer().await,
        "--using" => demo::using().await,
        "--force" => demo::force_terminating().await,
        "--reusing" => demo::reusing_hypervisor().await,
        "--syncusing" => demo::syncusing(),
        "--options" => demo::options().await,
        _ => panic!("unknown option"),
    };
}
