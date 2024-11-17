use actuate::prelude::*;
use actuate_core::use_task;
use serde::Deserialize;
use std::collections::HashMap;

#[derive(Deserialize)]
struct Response {
    message: HashMap<String, Vec<String>>,
}

#[derive(Data)]
struct App;

impl Compose for App {
    fn compose(cx: Scope<Self>) -> impl Compose {
        let breeds = use_mut(&cx, Vec::new);

        use_task(&cx, move || async move {
            let json: Response = reqwest::get("https://dog.ceo/api/breeds/list/all")
                .await
                .unwrap()
                .json()
                .await
                .unwrap();

            for (name, _) in json.message {
                breeds.update(|breeds| breeds.push(name));
            }
        });

        Window::new(compose::from_iter(breeds, |breed| Text::new(breed)))
    }
}

fn main() {
    actuate::run(App)
}
