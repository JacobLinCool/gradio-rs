use gradio::{Client, ClientOptions, PredictionInput};

fn main() {
    let client = Client::new_sync("gradio/hello_world", ClientOptions::default()).unwrap();

    let output = client
        .predict_sync("/predict", vec![PredictionInput::from_value("Jacob")])
        .unwrap();

    println!(
        "Output: {}",
        output[0].clone().as_value().unwrap().as_str().unwrap()
    );
}
