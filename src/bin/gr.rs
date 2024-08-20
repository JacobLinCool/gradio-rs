#[cfg(not(feature = "wasm"))]
use std::path::PathBuf;
#[cfg(not(feature = "wasm"))]
use anyhow::Result;
#[cfg(not(feature = "wasm"))]
use clap::{arg, Command};
#[cfg(not(feature = "wasm"))]
use gradio::{Client, ClientOptions, PredictionInput};


#[cfg(not(feature = "wasm"))]
#[tokio::main]
async fn main() -> Result<()> {
    let matches = cli().get_matches();

    let token = matches.get_one::<String>("token");
    let output = matches.get_one::<String>("output");

    match matches.subcommand() {
        Some(("run", sub_matches)) => {
            let space_id = sub_matches.get_one::<String>("space_id").expect("required");
            let route = sub_matches.get_one::<String>("route").expect("required");
            let options: Vec<&String> = sub_matches
                .get_many::<String>("options")
                .unwrap_or_default()
                .collect();
            run_command(space_id, route, options, token, output).await?;
        }
        Some(("list", sub_matches)) => {
            let space_id = sub_matches.get_one::<String>("space_id").expect("required");
            list_command(space_id, token).await?;
        }
        _ => {
            cli().print_help()?;
        }
    }

    Ok(())
}

#[cfg(not(feature = "wasm"))]
fn cli() -> Command {
    Command::new("gr")
        .version(env!("CARGO_PKG_VERSION"))
        .about("Gradio Command Line Client")
        .arg(arg!(-t --token <token> "The Hugging Face Access Token"))
        .arg(arg!(-o --output <output> "Output directory, if specified, files will be saved to this directory"))
        .subcommand_required(true)
        .arg_required_else_help(true)
        .subcommand(
            Command::new("run")
                .alias("r")
                .about("Perform a prediction")
                .arg(arg!(<space_id> "The ID of the Gradio space"))
                .arg(arg!(<route> "The route to run"))
                .arg(arg!([options]... "Options for the run command")),
        )
        .subcommand(
            Command::new("list")
                .alias("ls")
                .about("List routes in a Gradio app")
                .arg(arg!(<space_id> "The ID of the Gradio space")),
        )
}

#[cfg(not(feature = "wasm"))]
async fn run_command(
    space_id: &str,
    route: &str,
    options: Vec<&String>,
    token: Option<&String>,
    outdir: Option<&String>,
) -> Result<()> {
    let route = format!("/{}", route.trim_start_matches('/'));

    let opt = if let Some(token) = token {
        ClientOptions::with_hf_token(token.clone())
    } else {
        ClientOptions::default()
    };

    let client = Client::new(space_id, opt).await?;

    let spec = client.view_api();
    let endpoint = spec
        .named_endpoints
        .get(&route)
        .unwrap_or_else(|| panic!("Route {} not found", route));
    let parameters = &endpoint.parameters;

    let mut data = vec![];
    for (i, param) in parameters.iter().enumerate() {
        let value = options.get(i).unwrap_or_else(|| {
            panic!(
                "Missing parameter, expected {} parameters, got {}",
                parameters.len(),
                options.len()
            )
        });
        if param.python_type.r#type == "filepath" {
            data.push(PredictionInput::from_file(value));
        } else if param.r#type.r#type == "string" {
            data.push(PredictionInput::from_value(value));
        } else if param.r#type.r#type == "number" {
            if let Ok(value) = value.parse::<i64>() {
                data.push(PredictionInput::from_value(value));
            } else {
                data.push(PredictionInput::from_value(value.parse::<f64>()?));
            }
        } else if param.r#type.r#type == "boolean" {
            data.push(PredictionInput::from_value(value.parse::<bool>()?));
        } else {
            return Err(anyhow::anyhow!(
                "Unsupported parameter type: {}",
                param.r#type.r#type
            ));
        }
    }

    let http_client = client.http_client.clone();
    let output = client.predict(&route, data).await?;
    for (i, ret) in endpoint.returns.iter().enumerate() {
        let value = output.get(i).expect("Missing return value");
        let name = if let Some(label) = &ret.label {
            label
        } else if let Some(name) = &ret.parameter_name {
            name
        } else {
            "unnamed"
        };

        if value.is_file() {
            let file = value.clone().as_file()?;
            if let Some(outdir) = outdir {
                let mut fp = PathBuf::from(outdir);
                fp.push(format!("{}.{}", name, file.suggest_extension()));
                file.save_to_path(&fp, Some(http_client.clone())).await?;
                println!("{}: {}", name, fp.display());
            } else {
                println!("{}: {}", name, file.url.unwrap_or("".to_string()));
            }
        } else {
            println!("{}: {}", name, value.clone().as_value()?);
        }
    }

    Ok(())
}

#[cfg(not(feature = "wasm"))]
async fn list_command(space_id: &str, token: Option<&String>) -> Result<()> {
    let opt = if let Some(token) = token {
        ClientOptions::with_hf_token(token.clone())
    } else {
        ClientOptions::default()
    };

    let client = Client::new(space_id, opt).await?;

    let spec = client.view_api();
    println!("API Spec for {}:", space_id);
    for endpoint in spec.named_endpoints.keys() {
        println!("\t{}", endpoint);

        let parameters = &spec.named_endpoints[endpoint].parameters;
        println!("\t\tParameters:");
        for param in parameters {
            let name = if let Some(label) = &param.label {
                label
            } else if let Some(name) = &param.parameter_name {
                name
            } else {
                "unnamed"
            };
            println!(
                "\t\t\t{:20} ( {:8} ) {}",
                name, param.python_type.r#type, param.r#type.description
            );
        }

        let returns = &spec.named_endpoints[endpoint].returns;
        println!("\t\tReturns:");
        for ret in returns {
            let name = if let Some(label) = &ret.label {
                label
            } else if let Some(name) = &ret.parameter_name {
                name
            } else {
                "unnamed"
            };
            println!(
                "\t\t\t{:20} ( {:8} ) {}",
                name, ret.python_type.r#type, ret.r#type.description
            );
        }
    }

    Ok(())
}


#[cfg(feature = "wasm")]
fn main() {
    panic!("This binary is not supported for wasm.")
}
