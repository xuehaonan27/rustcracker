use std::env;

mod demo;

#[tokio::main]
async fn main() {
    env_logger::init();
    let set = env::args()
        .skip(1)
        .next()
        .expect("Must specify with demo to run");

    let _: () = match set.as_str() {
        "--no-jailer" => demo::no_jailer().await,
        "--with-jailer" => demo::with_jailer().await,
        "--reusing" => demo::reusing_hypervisor().await,
        _ => panic!("unknown option"),
    };
}
