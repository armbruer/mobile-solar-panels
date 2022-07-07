use embedded_svc::{
    ipv4,
    ping::Ping,
    wifi::{
        ApStatus, ClientConfiguration, ClientConnectionStatus, ClientIpStatus, ClientStatus,
        Configuration, Status, Wifi,
    },
};
use esp_idf_sys::EspError;
use log::info;

use esp_idf_svc::netif::EspNetifStack;
use esp_idf_svc::nvs::EspDefaultNvs;
use esp_idf_svc::ping;
use esp_idf_svc::sysloop::EspSysLoopStack;
use esp_idf_svc::wifi::EspWifi;

use std::sync::Arc;

const SSID: &str = env!("esp_wifi_ssid");
const PASS: &str = env!("esp_wifi_pass");

pub fn wifi(
    netif_stack: Arc<EspNetifStack>,
    sys_loop_stack: Arc<EspSysLoopStack>,
    default_nvs: Arc<EspDefaultNvs>,
) -> Result<Box<EspWifi>, EspError> {
    let mut wifi = Box::new(EspWifi::new(netif_stack, sys_loop_stack, default_nvs)?);

    wifi.set_configuration(&Configuration::Client(ClientConfiguration {
        ssid: SSID.into(),
        password: PASS.into(),
        channel: None,
        ..Default::default()
    }))?;

    info!("Wifi configuration set");
    Ok(wifi)
}

pub fn ping(ip_settings: &ipv4::ClientSettings) -> Result<(), EspError> {
    info!("About to do some pings for {:?}", ip_settings);

    let ping_summary =
        ping::EspPing::default().ping(ip_settings.subnet.gateway, &Default::default())?;
    if ping_summary.transmitted != ping_summary.received {
        todo!(
            // TODO
            "Pinging gateway {} resulted in timeouts",
            ip_settings.subnet.gateway
        )
    }

    info!("Pinging done");

    Ok(())
}

pub fn check_status(wifi: Box<EspWifi>) -> Result<(), EspError> {
    let status = wifi.get_status();

    if let Status(
        ClientStatus::Started(ClientConnectionStatus::Connected(ClientIpStatus::Done(ip_settings))),
        ApStatus::Stopped,
    ) = status
    {
        info!("Wifi connected");

        ping(&ip_settings)?;
        Ok(())
    } else {
        const ESP_ERR_INVALID_STATE: esp_idf_sys::esp_err_t = 0x103;
        Err(EspError::from(ESP_ERR_INVALID_STATE).unwrap())
    }
}
