use rustls::{server::ServerConfig, Certificate, PrivateKey};
use std::net::IpAddr;

fn self_signed_certificate(ip_addr: IpAddr) -> rcgen::Certificate {
    let subject_alt_names = vec![format!("{}", ip_addr)];
    rcgen::generate_simple_self_signed(subject_alt_names).unwrap()
}

pub fn server_config(ip_addr: IpAddr) -> ServerConfig {
    let self_signed_cert = self_signed_certificate(ip_addr);

    let certs = vec![Certificate(self_signed_cert.serialize_der().unwrap())];
    let private_key = PrivateKey(self_signed_cert.serialize_private_key_der());

    ServerConfig::builder()
        .with_safe_default_cipher_suites()
        .with_safe_default_kx_groups()
        .with_safe_default_protocol_versions()
        .unwrap()
        .with_no_client_auth()
        .with_single_cert(certs, private_key)
        .unwrap()
}
