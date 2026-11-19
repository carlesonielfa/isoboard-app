//! Opt-in API-key Keychain management for the external-provider probe.

use std::io::{self, IsTerminal, Read};

use isoboard_lib::api_credentials::{
    api_credential_store, validate_api_key, Provider, SecretStoreError,
};

#[derive(Debug)]
enum Error {
    Usage(&'static str),
    UnsupportedProvider,
    Input,
    SecretStore(SecretStoreError),
}

const MAX_API_KEY_BYTES: usize = 16 * 1024;

impl std::fmt::Display for Error {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Usage(message) => formatter.write_str(message),
            Self::UnsupportedProvider => formatter
                .write_str("provider must be one of: openai, anthropic, gemini, openai-compatible"),
            Self::Input => formatter.write_str("could not read an API key from stdin"),
            Self::SecretStore(error) => write!(formatter, "API-key storage failed: {error}"),
        }
    }
}

impl std::error::Error for Error {}

impl From<SecretStoreError> for Error {
    fn from(error: SecretStoreError) -> Self {
        Self::SecretStore(error)
    }
}

fn provider(value: &str) -> Result<Provider, Error> {
    Provider::parse(value).ok_or(Error::UnsupportedProvider)
}

fn usage() -> Error {
    Error::Usage(
        "usage: cargo run --features api-probe --example api_credentials -- <set|status|logout> PROVIDER",
    )
}

fn parse_args(args: &[String]) -> Result<(&str, Provider), Error> {
    if args.len() != 3 {
        return Err(usage());
    }
    Ok((&args[1], provider(&args[2])?))
}

fn read_secret() -> Result<Vec<u8>, Error> {
    if io::stdin().is_terminal() {
        let secret = rpassword::prompt_password("API key: ").map_err(|_| Error::Input)?;
        return validate_secret(secret.into_bytes());
    }

    let mut input = Vec::new();
    io::stdin()
        .take((MAX_API_KEY_BYTES + 1) as u64)
        .read_to_end(&mut input)
        .map_err(|_| Error::Input)?;
    if input.len() > MAX_API_KEY_BYTES {
        return Err(Error::Input);
    }
    while matches!(input.last(), Some(b'\n' | b'\r')) {
        input.pop();
    }
    validate_secret(input)
}

fn validate_secret(secret: Vec<u8>) -> Result<Vec<u8>, Error> {
    validate_api_key(&secret).map_err(|_| Error::Input)?;
    Ok(secret)
}

fn run(args: &[String]) -> Result<(), Error> {
    let (command, provider) = parse_args(args)?;
    let store = api_credential_store(provider)?;
    match command {
        "set" => {
            let secret = read_secret()?;
            validate_api_key(&secret).map_err(|_| Error::Input)?;
            store.set(&secret)?;
            println!("stored API key for {}", provider.id());
        }
        "status" => {
            let configured = match store.get()? {
                Some(secret) => {
                    validate_api_key(&secret)?;
                    true
                }
                None => false,
            };
            println!("configured={configured}");
        }
        "logout" => {
            store.delete()?;
            println!("removed API key for {}", provider.id());
        }
        _ => return Err(usage()),
    }
    Ok(())
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if let Err(error) = run(&args) {
        eprintln!("API credentials: {error}");
        std::process::exit(1);
    }
}
