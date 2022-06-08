use embedded_svc::{
    ipv4,
    ping::Ping,
    wifi::{
        AccessPointConfiguration, ApIpStatus, ApStatus, ClientConfiguration,
        ClientConnectionStatus, ClientIpStatus, ClientStatus, Configuration, Status, Wifi,
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
use std::time::Duration;

const SSID: &str = env!("RUST_ESP32_WIFI_SSID");
const PASS: &str = env!("RUST_ESP32_WIFI_PASS");

pub fn wifi(
    netif_stack: Arc<EspNetifStack>,
    sys_loop_stack: Arc<EspSysLoopStack>,
    default_nvs: Arc<EspDefaultNvs>,
) -> Result<Box<EspWifi>, EspError> {
    let mut wifi = Box::new(EspWifi::new(netif_stack, sys_loop_stack, default_nvs)?);

    info!("Wifi created, about to scan");

    let ap_infos = wifi.scan()?;

    let ours = ap_infos.into_iter().find(|a| a.ssid == SSID);

    let channel = if let Some(ours) = ours {
        info!(
            "Found configured access point {} on channel {}",
            SSID, ours.channel
        );
        Some(ours.channel)
    } else {
        info!(
            "Configured access point {} not found during scanning, will go with unknown channel",
            SSID
        );
        None
    };

    wifi.set_configuration(&Configuration::Mixed(
        ClientConfiguration {
            ssid: SSID.into(),
            password: PASS.into(),
            channel,
            ..Default::default()
        },
        AccessPointConfiguration {
            ssid: "aptest".into(),
            channel: channel.unwrap_or(1),
            ..Default::default()
        },
    ))?;

    info!("Wifi configuration set, about to get status");

    wifi.wait_status_with_timeout(Duration::from_secs(20), |status| !status.is_transitional())
        .map_err(|e| panic!("Unexpected Wifi status: {:?}", e))?; // TODO

    let status = wifi.get_status();

    if let Status(
        ClientStatus::Started(ClientConnectionStatus::Connected(ClientIpStatus::Done(ip_settings))),
        ApStatus::Started(ApIpStatus::Done),
    ) = status
    {
        info!("Wifi connected");

        ping(&ip_settings)?;
    } else {
        // TODO
        panic!("Unexpected Wifi status: {:?}", status);
    }

    Ok(wifi)
}

pub fn ping(ip_settings: &ipv4::ClientSettings) -> Result<(), EspError> {
    info!("About to do some pings for {:?}", ip_settings);

    let ping_summary =
        ping::EspPing::default().ping(ip_settings.subnet.gateway, &Default::default())?;
    if ping_summary.transmitted != ping_summary.received {
        panic!(
            // TODO
            "Pinging gateway {} resulted in timeouts",
            ip_settings.subnet.gateway
        )
    }

    info!("Pinging done");

    Ok(())
}
