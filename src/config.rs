use rocket::config::Config;

use std::net::{IpAddr, Ipv4Addr};

pub fn rocket_config() -> Config {
    let mut config = Config::default();
    config.address = IpAddr::V4(Ipv4Addr::UNSPECIFIED);
    config.port = 8080;
    // todo TLS
    //
    // https://docs.rs/rocket/0.5.0-rc.1/rocket/config/struct.TlsConfig.html
    // https://docs.rs/openssl/0.10.35/openssl/pkey/struct.PKey.html
    // must encrypt, embed pkey and cert as &[u8]
    config
}