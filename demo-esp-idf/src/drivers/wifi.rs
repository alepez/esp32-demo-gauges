use std::cell::RefCell;
use std::time::Duration;

use anyhow::bail;
use embedded_svc::wifi::{
    AccessPointConfiguration, AuthMethod, ClientConfiguration, Configuration,
};
use esp_idf_hal::modem::Modem;
use esp_idf_svc::eventloop::EspSystemEventLoop;
use esp_idf_svc::nvs::EspDefaultNvsPartition;
use esp_idf_svc::wifi::WifiWait;
use racegate::hal::wifi::{Wifi, WifiConfig};

pub struct EspWifi {
    esp_wifi: RefCell<esp_idf_svc::wifi::EspWifi<'static>>,
    sys_loop: EspSystemEventLoop,
}

fn to_esp_wifi_config(src: &WifiConfig) -> anyhow::Result<Configuration> {
    let &WifiConfig { ap, ssid, password } = src;

    if ssid.is_empty() {
        bail!("Wi-Fi SSID must be non-empty")
    }

    let auth_method = if password.is_empty() {
        log::info!("Wi-Fi password is empty. Authentication is disabled.");
        AuthMethod::None
    } else {
        AuthMethod::WPA2Personal
    };

    if ap {
        let config = AccessPointConfiguration {
            ssid: ssid.into(),
            password: password.into(),
            auth_method,
            ..Default::default()
        };

        Ok(Configuration::AccessPoint(config))
    } else {
        let config = ClientConfiguration {
            ssid: ssid.into(),
            password: password.into(),
            channel: Default::default(),
            auth_method,
            ..Default::default()
        };

        Ok(Configuration::Client(config))
    }
}

impl EspWifi {
    pub fn new(modem: Modem) -> anyhow::Result<EspWifi> {
        let sys_loop = EspSystemEventLoop::take()?;
        let nvs = EspDefaultNvsPartition::take()?;
        let esp_wifi = esp_idf_svc::wifi::EspWifi::new(modem, sys_loop.clone(), Some(nvs))?;
        Ok(Self {
            esp_wifi: RefCell::new(esp_wifi),
            sys_loop,
        })
    }
}

impl Wifi for EspWifi {
    fn setup(&self, config: &WifiConfig) -> anyhow::Result<()> {
        let is_access_point = config.ap;
        let config = to_esp_wifi_config(config)?;

        let mut esp_wifi = self.esp_wifi.try_borrow_mut()?;

        esp_wifi.set_configuration(&config)?;
        esp_wifi.start()?;

        let started = {
            let timeout = Duration::from_secs(20);
            let matcher = || esp_wifi.is_started().unwrap_or(false);
            WifiWait::new(&self.sys_loop)?.wait_with_timeout(timeout, matcher)
        };

        if !started {
            log::error!("Wi-Fi did not start");
        } else if !is_access_point {
            esp_wifi.connect()?;
        }

        Ok(())
    }

    fn is_up(&self) -> bool {
        if let Ok(esp_wifi) = self.esp_wifi.try_borrow() {
            esp_wifi.is_up().unwrap_or(false)
        } else {
            false
        }
    }

    fn reconnect(&self) {
        if let Ok(mut esp_wifi) = self.esp_wifi.try_borrow_mut() {
            esp_wifi.disconnect().expect("Cannot disconnect");
            esp_wifi.connect().expect("Cannot connect");
        }
    }
}
