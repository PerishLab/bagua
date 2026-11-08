use crate::error::Error;
use std::time::Duration;

pub(super) fn check(config: &bagua::config::Config) -> Result<(client::Keys, Duration), Error> {
    if config.version != 1 {
        return Err(Error::config("version: expected configuration version 1"));
    }
    config.history.check().map_err(|_| {
        Error::config(
            "history: lines must be 0..=50000, bytes <=67108864 and bytes <= total <=268435456",
        )
    })?;
    if config.client.escape > 1000 {
        return Err(Error::config("client.escape_ms: expected 0..=1000"));
    }
    let prefix = config.client.prefix.strip_prefix("C-").unwrap_or("");
    let mut chars = prefix.chars();
    let key = chars
        .next()
        .ok_or_else(|| Error::config("client.prefix: expected C-<letter>"))?;
    if chars.next().is_some() {
        return Err(Error::config("client.prefix: expected one control letter"));
    }
    let keys = client::Keys::new(key).map_err(|_| {
        Error::config(
            "client.prefix: use C-a..C-z except C-h/C-i/C-j/C-m, which decode as navigation keys",
        )
    })?;
    if config
        .host
        .socket
        .as_ref()
        .is_some_and(|path| !path.is_absolute())
    {
        return Err(Error::config("host.socket: expected an absolute path"));
    }
    if let Some(program) = &config.program {
        super::launch::check(program)?;
    }
    Ok((keys, Duration::from_millis(config.client.escape)))
}
