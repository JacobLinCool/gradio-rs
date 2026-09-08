use anyhow::Result;
use gradio::{Client, ClientOptions, PredictionInput};

const SAMPLE_AUDIO_URL: &str =
    "https://github.com/gradio-app/gradio/raw/main/test/test_files/audio_sample.wav";
const SAMPLE_AUDIO_PATH: &str = "/tmp/gradio-rs-live-audio-sample.wav";
const SAMPLE_MODEL_URL: &str =
    "https://raw.githubusercontent.com/gradio-app/gradio/main/gradio/media_assets/models3d/Fox.gltf";
const SAMPLE_MODEL_PATH: &str = "/tmp/gradio-rs-live-model-sample.gltf";
const I18N_APPS: [&str; 2] = [
    "https://voxcpm.modelbest.cn",
    "https://openbmb-voxcpm-demo.hf.space",
];

fn assert_i18n_label(client: &Client) {
    assert!(client.view_api().named_endpoints.values().any(|endpoint| {
        endpoint
            .parameters
            .iter()
            .any(|parameter| parameter.label.as_deref() == Some("show_prompt_text_label"))
    }));
}

#[tokio::test]
#[ignore = "requires live Gradio apps"]
async fn gradio_6_i18n_metadata_is_available() -> Result<()> {
    for app in I18N_APPS {
        let client = Client::new(app, ClientOptions::default()).await?;
        assert_i18n_label(&client);
    }
    Ok(())
}

#[test]
#[ignore = "requires live Gradio apps"]
fn gradio_6_i18n_metadata_is_available_sync() -> Result<()> {
    for app in I18N_APPS {
        let client = Client::new_sync(app, ClientOptions::default())?;
        assert_i18n_label(&client);
    }
    Ok(())
}

async fn ensure_sample_audio() -> Result<&'static str> {
    if tokio::fs::try_exists(SAMPLE_AUDIO_PATH).await? {
        return Ok(SAMPLE_AUDIO_PATH);
    }

    let bytes = reqwest::get(SAMPLE_AUDIO_URL).await?.bytes().await?;
    tokio::fs::write(SAMPLE_AUDIO_PATH, bytes).await?;
    Ok(SAMPLE_AUDIO_PATH)
}

async fn ensure_sample_model() -> Result<&'static str> {
    if tokio::fs::try_exists(SAMPLE_MODEL_PATH).await? {
        return Ok(SAMPLE_MODEL_PATH);
    }

    let bytes = reqwest::get(SAMPLE_MODEL_URL).await?.bytes().await?;
    tokio::fs::write(SAMPLE_MODEL_PATH, bytes).await?;
    Ok(SAMPLE_MODEL_PATH)
}

#[tokio::test]
#[ignore = "requires live Hugging Face Spaces"]
async fn gradio_6_space_id_predicts_successfully() -> Result<()> {
    let client = Client::new("gradio/hello_world", ClientOptions::default()).await?;
    let output = client
        .predict("/predict", vec![PredictionInput::from_value("Jacob")])
        .await?;

    assert_eq!(output[0].clone().as_value()?.as_str(), Some("Hello Jacob!"));
    Ok(())
}

#[tokio::test]
#[ignore = "requires live Hugging Face Spaces"]
async fn gradio_6_full_url_predicts_successfully() -> Result<()> {
    let client = Client::new(
        "https://gradio-hello-world.hf.space",
        ClientOptions::default(),
    )
    .await?;
    let output = client
        .predict("/predict", vec![PredictionInput::from_value("Rust")])
        .await?;

    assert_eq!(output[0].clone().as_value()?.as_str(), Some("Hello Rust!"));
    Ok(())
}

#[tokio::test]
#[ignore = "requires live Hugging Face Spaces"]
async fn gradio_6_file_upload_prediction_works() -> Result<()> {
    let sample_model = ensure_sample_model().await?;
    let client = Client::new("gradio/model3D", ClientOptions::default()).await?;
    let output = client
        .predict("/predict", vec![PredictionInput::from_file(sample_model)])
        .await?;

    let model = output[0].clone().as_file()?;
    assert_eq!(model.meta._type, "gradio.FileData");
    assert!(
        model
            .url
            .as_deref()
            .map(|url| !url.is_empty())
            .unwrap_or(false)
            || model
                .path
                .as_deref()
                .map(|path| !path.is_empty())
                .unwrap_or(false)
    );
    Ok(())
}

#[tokio::test]
#[ignore = "requires live Hugging Face Spaces"]
async fn gradio_5_file_upload_prediction_works() -> Result<()> {
    let sample_audio = ensure_sample_audio().await?;
    let client = Client::new("hf-audio/whisper-large-v3-turbo", ClientOptions::default()).await?;
    let output = client
        .predict(
            "/predict",
            vec![
                PredictionInput::from_file(sample_audio),
                PredictionInput::from_value("transcribe"),
            ],
        )
        .await?;

    let transcript = output[0].clone().as_value()?;
    assert!(transcript.as_str().unwrap_or("").trim().len() > 1);
    Ok(())
}

#[tokio::test]
#[ignore = "requires live Hugging Face Spaces"]
async fn gradio_4_space_metadata_is_available() -> Result<()> {
    let client = Client::new("JacobLinCool/vocal-separation", ClientOptions::default()).await?;
    let api = client.view_api();

    assert!(api.named_endpoints.contains_key("/separate"));
    Ok(())
}
