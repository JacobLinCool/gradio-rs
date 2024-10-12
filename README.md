# gradio-rs

Gradio Client in Rust.

![demo gif](./images/demo.gif)

## Features

- [x] View API
- [x] Upload file
- [x] Make prediction
  - [x] The blocking `predict` method
  - [x] The non-blocking `submit` method
- [x] Command-line interface
- [x] Synchronous and asynchronous API

> Supposed to work with Gradio 5 & 4, other versions are not tested.

## Documentation

- [API Documentation](https://docs.rs/gradio)
- [Examples](./examples/)

## Usage

See the [examples](./examples/) directory for more examples.

Here is an example of using `BS-RoFormer` model to separate vocals and background music from an audio file.

```rust
use gradio::{PredictionInput, Client, ClientOptions};

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
```

See [./examples/sd3.rs](./examples/sd3.rs) for non-blocking example with `submit` method.

## Command-line Interface

```sh
cargo install gradio
gr --help
```

Take `stabilityai/stable-diffusion-3-medium` HF Space as an example:

```sh
> gr list stabilityai/stable-diffusion-3-medium
API Spec for stabilityai/stable-diffusion-3-medium:
        /infer
                Parameters:
                        prompt               ( str      ) 
                        negative_prompt      ( str      ) 
                        seed                 ( float    ) numeric value between 0 and 2147483647
                        randomize_seed       ( bool     ) 
                        width                ( float    ) numeric value between 256 and 1344
                        height               ( float    ) numeric value between 256 and 1344
                        guidance_scale       ( float    ) numeric value between 0.0 and 10.0
                        num_inference_steps  ( float    ) numeric value between 1 and 50
                Returns:
                        Result               ( filepath ) 
                        Seed                 ( float    ) numeric value between 0 and 2147483647

> gr run stabilityai/stable-diffusion-3-medium infer 'Rusty text "AI & CLI" on the snow.' '' 0 true 1024 1024 5 28
Result: https://stabilityai-stable-diffusion-3-medium.hf.space/file=/tmp/gradio/5735ca7775e05f8d56d929d8f57b099a675c0a01/image.webp
Seed: 486085626
```

For file input, simply use the file path as the argument:

```sh
gr run hf-audio/whisper-large-v3 predict 'test-audio.wav' 'transcribe'
output: " Did you know you can try the coolest model on your command line?"
```
