pub trait Wifi {
    fn setup(&self, config: &WifiConfig) -> anyhow::Result<()>;

    fn is_up(&self) -> bool;

    fn reconnect(&self);
}

#[derive(Eq, PartialEq)]
pub struct WifiConfig<'a> {
    pub ap: bool,
    pub ssid: &'a str,
    pub password: &'a str,
}

pub enum WifiConfigError {
    EnvVarNotAvailable,
    ParseError,
}

impl WifiConfig<'_> {
    fn try_from_str(s: &'static str) -> Result<Self, WifiConfigError> {
        let mut iter = s.split_terminator(':');
        let ap: bool = iter
            .next()
            .ok_or(WifiConfigError::ParseError)?
            .parse()
            .or(Err(WifiConfigError::ParseError))?;
        let ssid: &str = iter.next().ok_or(WifiConfigError::ParseError)?;
        let password: &str = iter.next().ok_or(WifiConfigError::ParseError)?;
        Ok(WifiConfig { ap, ssid, password })
    }

    pub fn from_env_var() -> Result<Self, WifiConfigError> {
        if let Some(s) = option_env!("RACEGATE_WIFI_CONFIG") {
            WifiConfig::try_from_str(s)
        } else {
            Err(WifiConfigError::EnvVarNotAvailable)
        }
    }
}

impl Default for WifiConfig<'_> {
    fn default() -> Self {
        WifiConfig {
            ap: true,
            ssid: "racegate",
            password: "racegate",
        }
    }
}
