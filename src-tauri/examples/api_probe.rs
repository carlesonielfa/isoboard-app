//! Opt-in external API provider probe.

use std::io::Cursor;

use image::{DynamicImage, ImageBuffer, ImageFormat, Rgb};
use isoboard_lib::api_credentials::{api_credential_store, validate_api_key, SecretStoreError};
use isoboard_lib::inference::{ApiInference, InferenceConfig, InferenceError, Provider};
use tokio_util::sync::CancellationToken;

const EXPECTED_TEXT: &str = "ISOBOARD 123";

#[derive(Debug)]
enum Error {
    Usage(&'static str),
    UnsupportedProvider,
    MissingApiKey,
    InvalidApiKey,
    Image,
    SecretStore(SecretStoreError),
    Inference(InferenceError),
}

impl std::fmt::Display for Error {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Usage(message) => formatter.write_str(message),
            Self::UnsupportedProvider => formatter
                .write_str("provider must be one of: openai, anthropic, gemini, openai-compatible"),
            Self::MissingApiKey => {
                formatter.write_str("API key is not configured; use api-key-set")
            }
            Self::InvalidApiKey => formatter.write_str("stored API key is invalid"),
            Self::Image => formatter.write_str("could not create synthetic probe image"),
            Self::SecretStore(error) => write!(formatter, "API-key storage failed: {error}"),
            Self::Inference(error) => write!(formatter, "API inference failed: {error}"),
        }
    }
}

impl std::error::Error for Error {}

impl From<SecretStoreError> for Error {
    fn from(error: SecretStoreError) -> Self {
        Self::SecretStore(error)
    }
}

impl From<InferenceError> for Error {
    fn from(error: InferenceError) -> Self {
        Self::Inference(error)
    }
}

fn parse_provider(value: &str) -> Result<Provider, Error> {
    Provider::parse(value).ok_or(Error::UnsupportedProvider)
}

fn parse_args(args: &[String]) -> Result<(Provider, &str, Option<&str>), Error> {
    if args.len() < 4 || args.len() > 5 || args[1] != "probe" || args[3].trim().is_empty() {
        return Err(Error::Usage(
            "usage: cargo run --features api-probe --example api_probe -- probe PROVIDER MODEL [ENDPOINT]",
        ));
    }
    let provider = parse_provider(&args[2])?;
    let endpoint = args.get(4).and_then(|value| {
        let value = value.trim();
        (!value.is_empty()).then_some(value)
    });
    if provider == Provider::OpenAiCompatible && endpoint.is_none() {
        return Err(Error::Usage(
            "openai-compatible requires an explicit ENDPOINT",
        ));
    }
    Ok((provider, &args[3], endpoint))
}

fn synthetic_image() -> Result<Vec<u8>, Error> {
    let mut image = ImageBuffer::from_pixel(640, 160, Rgb([255_u8, 255, 255]));
    draw_text(&mut image, EXPECTED_TEXT, 32, 48, 8);
    let mut bytes = Cursor::new(Vec::new());
    DynamicImage::ImageRgb8(image)
        .write_to(&mut bytes, ImageFormat::Png)
        .map_err(|_| Error::Image)?;
    Ok(bytes.into_inner())
}

fn draw_text(image: &mut ImageBuffer<Rgb<u8>, Vec<u8>>, text: &str, x: u32, y: u32, scale: u32) {
    for (index, character) in text.chars().enumerate() {
        let glyph = glyph(character);
        let origin_x = x + index as u32 * 6 * scale;
        for (row, bits) in glyph.iter().enumerate() {
            for column in 0..5 {
                if bits & (1 << (4 - column)) == 0 {
                    continue;
                }
                for dy in 0..scale {
                    for dx in 0..scale {
                        image.put_pixel(
                            origin_x + column * scale + dx,
                            y + row as u32 * scale + dy,
                            Rgb([24, 24, 24]),
                        );
                    }
                }
            }
        }
    }
}

fn glyph(character: char) -> [u8; 7] {
    match character {
        'I' => [
            0b11111, 0b00100, 0b00100, 0b00100, 0b00100, 0b00100, 0b11111,
        ],
        'S' => [
            0b11111, 0b10000, 0b10000, 0b11111, 0b00001, 0b00001, 0b11111,
        ],
        'O' => [
            0b01110, 0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b01110,
        ],
        'B' => [
            0b11110, 0b10001, 0b10001, 0b11110, 0b10001, 0b10001, 0b11110,
        ],
        'A' => [
            0b01110, 0b10001, 0b10001, 0b11111, 0b10001, 0b10001, 0b10001,
        ],
        'R' => [
            0b11110, 0b10001, 0b10001, 0b11110, 0b10100, 0b10010, 0b10001,
        ],
        'D' => [
            0b11110, 0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b11110,
        ],
        '1' => [
            0b00100, 0b01100, 0b00100, 0b00100, 0b00100, 0b00100, 0b01110,
        ],
        '2' => [
            0b01110, 0b10001, 0b00001, 0b00010, 0b00100, 0b01000, 0b11111,
        ],
        '3' => [
            0b11110, 0b00001, 0b00001, 0b01110, 0b00001, 0b00001, 0b11110,
        ],
        _ => [0; 7],
    }
}

fn cancellation_token() -> CancellationToken {
    let token = CancellationToken::new();
    let waiter = token.clone();
    tokio::spawn(async move {
        if tokio::signal::ctrl_c().await.is_ok() {
            waiter.cancel();
        }
    });
    token
}

#[tokio::main]
async fn main() {
    let args: Vec<String> = std::env::args().collect();
    if let Err(error) = run(&args).await {
        eprintln!("API probe: {error}");
        std::process::exit(1);
    }
}

async fn run(args: &[String]) -> Result<(), Error> {
    let (provider, model, endpoint) = parse_args(args)?;
    let store = api_credential_store(provider)?;
    // Self-hosted OpenAI-compatible servers may run without a key; the probe
    // then sends no Authorization header.
    let key = match store.get()? {
        Some(key) => {
            validate_api_key(&key).map_err(|_| Error::InvalidApiKey)?;
            String::from_utf8(key).map_err(|_| Error::InvalidApiKey)?
        }
        None if provider == Provider::OpenAiCompatible => String::new(),
        None => return Err(Error::MissingApiKey),
    };
    let image = synthetic_image()?;
    let inference = ApiInference::new().map_err(Error::from)?;
    let mut config = InferenceConfig::new(provider, model, key);
    if let Some(endpoint) = endpoint {
        config = config.with_base_url(endpoint);
    }
    let output = inference
        .extract(&config, &image, "image/png", cancellation_token())
        .await
        .map_err(Error::from)?;
    println!("{output}");
    println!(
        "expected_text_present={}",
        output.to_ascii_uppercase().contains(EXPECTED_TEXT)
    );
    Ok(())
}
