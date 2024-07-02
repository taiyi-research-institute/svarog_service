use clap::{value_parser, Arg, ArgAction, Command};
use erreur::*;
use svarog_grpc::mpc_session_manager_server::{
    MpcSessionManagerServer, // server struct
};
use tonic::transport::{Identity, Server, ServerTlsConfig};

mod server_impl;
pub use server_impl::*;

#[tokio::main]
async fn main() -> Resultat<()> {
    // Parse args
    let matches = Command::new("svarog_sesman")
        .arg(
            Arg::new("host")
                .short('h')
                .required(false)
                .default_value("0.0.0.0")
                .action(ArgAction::Set),
        )
        .arg(
            Arg::new("port")
                .short('p')
                .required(false)
                .default_value("2000")
                .value_parser(value_parser!(u16))
                .action(ArgAction::Set),
        )
        .arg(Arg::new("https").long("https").action(ArgAction::SetTrue))
        .disable_help_flag(true)
        .get_matches();
    let host: String = matches.get_one::<String>("host").ifnone_()?.to_owned();
    let port: u16 = matches.get_one::<u16>("port").ifnone_()?.to_owned();
    let https: bool = matches.get_flag("https");
    println!("{}", svarog_sesman::version());
    println!("svarog_sesman will listen on {}:{}", &host, port);

    // Init service
    let (sesman, recycle_task_handle) = Sesman::init().await.catch_()?;

    // Start server
    let mut server = Server::builder();
    if https {
        let cert = tokio::fs::read_to_string("tls/cert.pem").await.catch_()?;
        let key = tokio::fs::read_to_string("tls/privkey.pem")
            .await
            .catch_()?;
        let ident = Identity::from_pem(cert, key);
        server = server
            .tls_config(ServerTlsConfig::new().identity(ident))
            .catch_()?;
    }
    server
        .add_service(MpcSessionManagerServer::new(sesman))
        .serve(format!("{host}:{port}").parse().unwrap())
        .await
        .catch("GrpcServerIsDown", "MpcSessionManager")?;

    recycle_task_handle.abort();

    Ok(())
}
