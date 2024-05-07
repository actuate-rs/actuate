use actuate::View;

fn app() -> impl View {
    actuate::from_fn(|| {
        actuate::from_fn(|| {
            dbg!("Hello World!");
        })
    })
}

#[tokio::main]
async fn main() {
    actuate::run(app()).await;
}
