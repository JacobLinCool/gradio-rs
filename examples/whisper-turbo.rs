use gradio::{Client, ClientOptions, PredictionInput};

#[tokio::main]
async fn main() {
    if std::env::args().len() < 2 {
        println!("Please provide an audio file path as an argument");
        std::process::exit(1);
    }
    let args: Vec<String> = std::env::args().collect();
    let file_path = &args[1];
    println!("File: {}", file_path);

    // Gradio v5
    let client = Client::new("hf-audio/whisper-large-v3-turbo", ClientOptions::default())
        .await
        .unwrap();

    let output = client
        .predict(
            "/predict",
            vec![
                PredictionInput::from_file(file_path),
                PredictionInput::from_value("transcribe"),
            ],
        )
        .await
        .unwrap();
    println!(
        "Output: {}",
        output[0].clone().as_value().unwrap().as_str().unwrap()
    );
}
