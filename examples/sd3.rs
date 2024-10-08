// cargo run --example sd3 -- 'A cute Crab in gradient colors.'
use gradio::{Client, ClientOptions, PredictionInput, PredictionOutput};

#[tokio::main]
async fn main() {
    if std::env::args().len() < 2 {
        println!("Please provide the prompt as an argument");
        std::process::exit(1);
    }
    let args: Vec<String> = std::env::args().collect();
    let prompt = &args[1];

    let client = Client::new(
        "stabilityai/stable-diffusion-3-medium",
        ClientOptions::default(),
    )
    .await
    .unwrap();

    let mut prediction = client
        .submit(
            "/infer",
            vec![
                PredictionInput::from_value(prompt),
                PredictionInput::from_value(""),   // negative_prompt
                PredictionInput::from_value(0),    // seed
                PredictionInput::from_value(true), // randomize_seed
                PredictionInput::from_value(1024), // width
                PredictionInput::from_value(1024), // height
                PredictionInput::from_value(5),    // guidance_scale
                PredictionInput::from_value(28),   // num_inference_steps
            ],
        )
        .await
        .unwrap();

    while let Some(event) = prediction.next().await {
        let event = event.unwrap();
        match event {
            gradio::structs::QueueDataMessage::Estimation {
                rank, queue_size, ..
            } => {
                println!("Queueing: {}/{}", rank + 1, queue_size);
            }
            gradio::structs::QueueDataMessage::Progress { progress_data, .. } => {
                if progress_data.is_none() {
                    continue;
                }
                let progress_data = progress_data.unwrap();
                if !progress_data.is_empty() {
                    let progress_data = &progress_data[0];
                    println!(
                        "Processing: {}/{} {}",
                        progress_data.index + 1,
                        progress_data.length.unwrap(),
                        progress_data.unit
                    );
                }
            }
            gradio::structs::QueueDataMessage::ProcessCompleted { output, .. } => {
                let output: Vec<PredictionOutput> = output.try_into().unwrap();

                println!(
                    "Generated Image: {}",
                    output[0].clone().as_file().unwrap().url.unwrap()
                );
                println!(
                    "Seed: {}",
                    output[1].clone().as_value().unwrap().as_i64().unwrap()
                );
                break;
            }
            _ => {}
        }
    }
}
