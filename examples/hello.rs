use gradio::{Client, ClientOptions, PredictionInput};

#[tokio::main]
async fn main() {
    let client = Client::new("gradio/hello_world", ClientOptions::default())
        .await
        .unwrap();

    let output = client
        .predict("/predict", vec![PredictionInput::from_value("Jacob")])
        .await
        .unwrap();
    println!(
        "Output: {}",
        output[0].clone().as_value().unwrap().as_str().unwrap()
    );
}
