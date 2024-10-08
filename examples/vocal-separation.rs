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

    let client = Client::new("JacobLinCool/vocal-separation", ClientOptions::default())
        .await
        .unwrap();

    let output = client
        .predict(
            "/separate",
            vec![
                PredictionInput::from_file(file_path),
                PredictionInput::from_value("BS-RoFormer"),
            ],
        )
        .await
        .unwrap();
    println!(
        "Vocals: {}",
        output[0].clone().as_file().unwrap().url.unwrap()
    );
    println!(
        "Background: {}",
        output[1].clone().as_file().unwrap().url.unwrap()
    );
}
