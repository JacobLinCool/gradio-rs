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

> Supposed to work with Gradio 4, 5, and 6, other versions are not tested.

## Documentation

- [API Documentation](https://docs.rs/gradio)
- [Examples](./examples/)

## Usage

See the [examples](./examples/) directory for more examples.

Here is an example of using `BS-RoFormer` model to separate vocals and background music from an audio file.

```rust
use gradio::{PredictionInput, Client, ClientOptions, Result};

#[tokio::main]
async fn main() -> Result<()> {
    if std::env::args().len() < 2 {
        println!("Please provide an audio file path as an argument");
        std::process::exit(1);
    }
    let args: Vec<String> = std::env::args().collect();
    let file_path = &args[1];
    println!("File: {}", file_path);

    let client = Client::new("JacobLinCool/vocal-separation", ClientOptions::default())
        .await?;

    let output = client
        .predict(
            "/separate",
            vec![
                PredictionInput::from_file(file_path),
                PredictionInput::from_value("BS-RoFormer"),
            ],
        )
        .await?;
    println!(
        "Vocals: {}",
        output[0].clone().as_file().unwrap().url.unwrap()
    );
    println!(
        "Background: {}",
        output[1].clone().as_file().unwrap().url.unwrap()
    );

    Ok(())
}
```

See [./examples/sd3.rs](./examples/sd3.rs) for non-blocking example with `submit` method.

## Errors

The library now exposes `gradio::Error` and `gradio::Result<T>` as its primary error model.

```rust
use gradio::{Client, ClientOptions, Error, PredictionInput, Result};

#[tokio::main]
async fn main() -> Result<()> {
    let client = Client::new("gradio/hello_world", ClientOptions::default()).await?;

    match client.predict("/missing", vec![PredictionInput::from_value("Rust")]).await {
        Ok(output) => {
            println!("{:?}", output);
            Ok(())
        }
        Err(Error::InvalidRoute { route }) => {
            eprintln!("Invalid route: {}", route);
            Ok(())
        }
        Err(err) => Err(err),
    }
}
```

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
gr run hf-audio/whisper-large-v3 transcribe 'test-audio.wav' 'transcribe'
output: " Did you know you can try the coolest model on your command line?"
```
