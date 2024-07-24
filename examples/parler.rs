use gradio::{Client, ClientOptions, PredictionInput};

#[tokio::main]
async fn main() {
    if std::env::args().len() < 2 {
        println!("Please provide the content as an argument");
        std::process::exit(1);
    }
    let args: Vec<String> = std::env::args().collect();
    let content = &args[1];
    let description = if args.len() > 2 {
        &args[2]
    } else {
        "Talia speaks high quality audio."
    };

    let client = Client::new("parler-tts/parler-tts-expresso", ClientOptions::default())
        .await
        .unwrap();

    let output = client
        .predict(
            "/gen_tts",
            vec![
                PredictionInput::from_value(content),
                PredictionInput::from_value(description),
            ],
        )
        .await
        .unwrap();
    println!(
        "Generated audio: {}",
        output[0].clone().as_file().unwrap().url.unwrap()
    );
}
