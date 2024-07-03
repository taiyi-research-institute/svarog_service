use std::time::Duration;

use erreur::*;
use svarog_grpc::{
    mpc_peer_client::MpcPeerClient as Peer,
    mpc_session_manager_client::MpcSessionManagerClient as Sesman, Void,
};
use tonic::{
    transport::{Certificate, Channel, ClientTlsConfig},
    Request,
};

#[tokio::main]
async fn main() -> Resultat<()> {
    // Parse args
    use clap::{Arg, ArgAction, Command};
    let matches = Command::new("svarog_ping")
        .arg(
            Arg::new("peer")
                .long("peer")
                .required(true)
                .action(ArgAction::Set),
        )
        .arg(
            Arg::new("man")
                .long("man")
                .required(true)
                .action(ArgAction::Set),
        )
        .arg(
            Arg::new("ca")
                .long("ca")
                .required(false)
                .default_value("")
                .action(ArgAction::Set),
        )
        .get_matches();
    let peer_url: String = matches.get_one::<String>("peer").ifnone_()?.to_owned();
    let man_url: String = matches.get_one::<String>("man").ifnone_()?.to_owned();
    let ca_path: String = matches.get_one::<String>("man").ifnone_()?.to_owned();

    let peer_https: bool;
    if peer_url.starts_with("https://") {
        peer_https = true;
    } else if peer_url.starts_with("http://") {
        peer_https = false;
    } else {
        throw!("", "Invalid peer url");
    };

    let man_https: bool;
    if peer_url.starts_with("https://") {
        man_https = true;
    } else if peer_url.starts_with("http://") {
        man_https = false;
    } else {
        throw!("", "Invalid peer url");
    };

    let mut peer = {
        let mut ch = Channel::from_shared(peer_url.to_string()).catch_()?;
        if peer_https {
            let pem = tokio::fs::read_to_string(&ca_path).await.catch_()?;
            let ca = Certificate::from_pem(pem);
            let tls = ClientTlsConfig::new().ca_certificate(ca);
            ch = ch.tls_config(tls).catch_()?;
        }
        let ch = ch
            .connect()
            .await
            .catch("", format!("Try connecting to {}", peer_url))?;
        Peer::new(ch)
    };

    let mut man = {
        let mut ch = Channel::from_shared(man_url.to_string()).catch_()?;
        if man_https {
            let pem = tokio::fs::read_to_string(&ca_path).await.catch_()?;
            let ca = Certificate::from_pem(pem);
            let tls = ClientTlsConfig::new().ca_certificate(ca);
            ch = ch.tls_config(tls).catch_()?;
        }
        let ch = ch
            .connect()
            .await
            .catch("", format!("Try connecting to {}", man_url))?;
        Sesman::new(ch)
    };

    let mut req = Request::new(Void {});
    req.set_timeout(Duration::from_secs(30));
    let resp = peer.ping(req).await.catch_()?.into_inner().value;
    println!("{}", resp);

    let mut req = Request::new(Void {});
    req.set_timeout(Duration::from_secs(30));
    let resp = man.ping(req).await.catch_()?.into_inner().value;
    println!("{}", resp);

    Ok(())
}
