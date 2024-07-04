/// 模块职责:
/// 1. 将player名称转换为数字id
/// 2. 将Keystore和Signature转换为内部数据结构, 以及反向转换.
/// 3. 调用算法实现.
use std::collections::{BTreeMap, BTreeSet, HashMap};

use erreur::*;
use svarog_grpc::{
    mpc_peer_server::MpcPeer, Curve, Department, EchoMessage, KeyTag, ParamsKeygen,
    ParamsKeygenMnem, ParamsReshare, ParamsSign, Scheme, SessionConfig, SessionId, Signature,
    VecSignature, Void,
};
use svarog_sesman::SvarogChannel;
use tonic::{Request, Response, Status};

use crate::server_impl::biz::{keygen_mnem_taproot, keygen_taproot, reshare_taproot, sign_taproot};

use super::biz::{
    keygen_frost, keygen_gg18, keygen_mnem_frost, keygen_mnem_gg18, reshare_frost, reshare_gg18,
    sign_frost, sign_gg18,
};

#[derive(Clone, Copy)]
pub(crate) struct SvarogPeer;

#[derive(Debug)]
pub(crate) struct SessionArchitecture {
    i_dict: BTreeMap<usize, usize>,
    th_dict: BTreeMap<usize, usize>,
    players: BTreeMap<usize, BTreeSet<usize>>,
}

fn ses_arch(
    name: &str,
    names: &HashMap<String, Department>,
    root_th: usize,
) -> SessionArchitecture {
    let mut ths: BTreeMap<String, usize> = names
        .iter()
        .map(|(k, v)| (k.clone(), v.threshold as usize))
        .collect();
    ths.insert("".to_owned(), root_th as usize);
    let mut dept_j = 0;
    let mut th_dict = BTreeMap::new();
    for (dept_name, _dept) in ths.iter() {
        dept_j += 1;
        th_dict.insert(dept_j, ths[dept_name]);
    }

    let mut names_dict: BTreeMap<String, BTreeMap<String, bool>> = BTreeMap::new();
    for (dept_name, dept) in names.iter() {
        let mut dept_players = BTreeMap::new();
        for (player, att) in dept.players.iter() {
            dept_players.insert(player.clone(), *att);
        }
        names_dict.insert(dept_name.clone(), dept_players);
    }
    let mut root_dept: BTreeMap<String, bool> = BTreeMap::new();
    for dept in names_dict.values() {
        for (name, att) in dept.iter() {
            if root_dept.contains_key(name) {
                let entry = root_dept.get_mut(name).unwrap();
                *entry = *entry || *att;
            } else {
                root_dept.insert(name.clone(), *att);
            }
        }
    }
    names_dict.insert("".to_owned(), root_dept);

    let mut i_dict = BTreeMap::new();
    let mut players: BTreeMap<usize, BTreeSet<usize>> = BTreeMap::new();
    let mut j = 0;
    let mut dept_j = 0;
    for (_dept_name, dept) in names_dict.iter() {
        dept_j += 1;
        for (player, &att) in dept.iter() {
            j += 1;
            if att {
                players
                    .entry(dept_j)
                    .or_insert(Default::default())
                    .insert(j);
                if name == player {
                    i_dict.insert(j, dept_j);
                }
            }
        }
    }
    SessionArchitecture {
        i_dict,
        th_dict,
        players,
    }
}

#[tonic::async_trait]
impl MpcPeer for SvarogPeer {
    async fn new_session(
        &self,
        request: Request<SessionConfig>,
    ) -> Result<Response<SessionId>, Status> {
        let cfg = request.into_inner();
        let chan = SvarogChannel::new_session(&cfg, &cfg.sesman_url, false)
            .await
            .catch_()
            .map_err(|e| Status::internal(e.to_string()))?;
        let tag = SessionId {
            value: chan.sid().to_owned(),
        };
        Ok(Response::new(tag))
    }

    async fn keygen(&self, request: Request<ParamsKeygen>) -> Result<Response<KeyTag>, Status> {
        async fn foo(request: Request<ParamsKeygen>) -> Resultat<KeyTag> {
            let params = request.into_inner();
            let (chan, cfg) =
                SvarogChannel::use_session(&params.session_id, &params.sesman_url, false)
                    .await
                    .catch_()?;
            let arch = ses_arch(&params.member_name, &cfg.players, cfg.threshold as usize);
            '_assert: {
                let num_players: usize = cfg.players.iter().map(|(_, v)| v.players.len()).sum();
                let num_attending: usize = arch.players.iter().map(|(_, v)| v.len()).sum();
                let num_attending = num_attending / 2;

                assert_throw!(
                    num_players == num_attending,
                    "all keygen players should attend"
                );
            }
            let algo = cfg.algorithm.ifnone_()?;
            let curve = algo.curve();
            let scheme = algo.scheme();
            let keytag = match (curve, scheme) {
                (Curve::Secp256k1, Scheme::ElGamal) => keygen_gg18(
                    chan,
                    params.member_name.clone(),
                    arch.i_dict,
                    arch.th_dict,
                    arch.players,
                )
                .await
                .catch_()?,
                (Curve::Ed25519, Scheme::Schnorr) => keygen_frost(
                    chan,
                    params.member_name.clone(),
                    arch.i_dict,
                    arch.th_dict,
                    arch.players,
                )
                .await
                .catch_()?,
                (Curve::Secp256k1, Scheme::Schnorr) => keygen_taproot(
                    chan,
                    params.member_name.clone(),
                    arch.i_dict,
                    arch.th_dict,
                    arch.players,
                )
                .await
                .catch_()?,
                _ => {
                    let msg = format!("Combination of curve {:?} and scheme {:?}.", curve, scheme);
                    throw!("NotImplemented", msg);
                }
            };
            Ok(keytag)
        }
        let keytag = foo(request)
            .await
            .catch_()
            .map_err(|e| Status::internal(e.to_string()))?;

        Ok(Response::new(keytag))
    }

    async fn keygen_mnem(
        &self,
        request: Request<ParamsKeygenMnem>,
    ) -> Result<Response<KeyTag>, Status> {
        async fn foo(request: Request<ParamsKeygenMnem>) -> Resultat<KeyTag> {
            let params = request.into_inner();
            let (chan, cfg) =
                SvarogChannel::use_session(&params.session_id, &params.sesman_url, false)
                    .await
                    .catch_()?;
            let arch = ses_arch(&params.member_name, &cfg.players, cfg.threshold as usize);
            '_assert: {
                let num_players: usize = cfg.players.iter().map(|(_, v)| v.players.len()).sum();
                let num_attending: usize = arch.players.iter().map(|(_, v)| v.len()).sum();
                let num_attending = num_attending / 2;
                assert_throw!(
                    num_players == num_attending,
                    "all keygen players should attend"
                );
            }
            let algo = cfg.algorithm.ifnone_()?;
            let curve = algo.curve();
            let scheme = algo.scheme();
            let keytag = match (curve, scheme) {
                (Curve::Secp256k1, Scheme::ElGamal) => keygen_mnem_gg18(
                    chan,
                    params.member_name,
                    arch.i_dict,
                    arch.th_dict,
                    arch.players,
                    params.mnemonic,
                )
                .await
                .catch_()?,
                (Curve::Ed25519, Scheme::Schnorr) => keygen_mnem_frost(
                    chan,
                    params.member_name,
                    arch.i_dict,
                    arch.th_dict,
                    arch.players,
                    params.mnemonic,
                )
                .await
                .catch_()?,
                (Curve::Secp256k1, Scheme::Schnorr) => keygen_mnem_taproot(
                    chan,
                    params.member_name,
                    arch.i_dict,
                    arch.th_dict,
                    arch.players,
                    params.mnemonic,
                )
                .await
                .catch_()?,
                _ => {
                    let msg = format!("Combination of curve {:?} and scheme {:?}.", curve, scheme);
                    throw!("NotImplemented", msg);
                }
            };
            Ok(keytag)
        }
        let keytag = foo(request)
            .await
            .catch_()
            .map_err(|e| Status::internal(e.to_string()))?;
        Ok(Response::new(keytag))
    }

    async fn sign(&self, request: Request<ParamsSign>) -> Result<Response<VecSignature>, Status> {
        async fn foo(request: Request<ParamsSign>) -> Resultat<Vec<Signature>> {
            let params = request.into_inner();
            let (chan, cfg) =
                SvarogChannel::use_session(&params.session_id, &params.sesman_url, false)
                    .await
                    .catch_()?;
            let arch = ses_arch("", &cfg.players, 0);
            let tasks = params.tasks;
            let algo = cfg.algorithm.ifnone_()?;
            let curve = algo.curve();
            let scheme = algo.scheme();
            let sigs = match (curve, scheme) {
                (Curve::Secp256k1, Scheme::ElGamal) => {
                    sign_gg18(chan, params.member_name, params.key_id, arch.players, tasks)
                        .await
                        .catch_()?
                }
                (Curve::Ed25519, Scheme::Schnorr) => {
                    sign_frost(chan, params.member_name, params.key_id, arch.players, tasks)
                        .await
                        .catch_()?
                }
                (Curve::Secp256k1, Scheme::Schnorr) => {
                    sign_taproot(chan, params.member_name, params.key_id, arch.players, tasks)
                        .await
                        .catch_()?
                }
                _ => {
                    let msg = format!("Combination of curve {:?} and scheme {:?}.", curve, scheme);
                    throw!("NotImplemented", msg);
                }
            };
            Ok(sigs)
        }
        let sigs = foo(request)
            .await
            .catch_()
            .map_err(|e| Status::internal(e.to_string()))?;
        let sigs = VecSignature { values: sigs };
        Ok(Response::new(sigs))
    }

    async fn reshare(&self, request: Request<ParamsReshare>) -> Result<Response<KeyTag>, Status> {
        async fn foo(request: Request<ParamsReshare>) -> Resultat<KeyTag> {
            let params = request.into_inner();
            let (chan, cfg) =
                SvarogChannel::use_session(&params.session_id, &params.sesman_url, false)
                    .await
                    .catch_()?;
            let providers = ses_arch("", &cfg.players, 0).players;
            let arch = ses_arch(
                &params.member_name,
                &cfg.players_reshared,
                cfg.threshold as usize,
            );
            let i_dict = arch.i_dict;
            let th_dict = arch.th_dict;
            let consumers = arch.players;
            '_assert: {
                let num_players: usize = cfg
                    .players_reshared
                    .iter()
                    .map(|(_, v)| v.players.len())
                    .sum();
                let num_attending: usize = consumers.iter().map(|(_, v)| v.len()).sum();
                let num_attending = num_attending / 2;
                assert_throw!(
                    num_players == num_attending,
                    "all reshare consumers should attend"
                );
            }
            let algo = cfg.algorithm.ifnone_()?;
            let curve = algo.curve();
            let scheme = algo.scheme();
            let keytag = match (curve, scheme) {
                (Curve::Secp256k1, Scheme::ElGamal) => reshare_gg18(
                    chan,
                    params.member_name,
                    params.key_id,
                    i_dict,
                    th_dict,
                    providers,
                    consumers,
                )
                .await
                .catch_()?,
                (Curve::Ed25519, Scheme::Schnorr) => reshare_frost(
                    chan,
                    params.member_name,
                    params.key_id,
                    i_dict,
                    th_dict,
                    providers,
                    consumers,
                )
                .await
                .catch_()?,
                (Curve::Secp256k1, Scheme::Schnorr) => reshare_taproot(
                    chan,
                    params.member_name,
                    params.key_id,
                    i_dict,
                    th_dict,
                    providers,
                    consumers,
                )
                .await
                .catch_()?,
                _ => {
                    let msg = format!("Combination of curve {:?} and scheme {:?}.", curve, scheme);
                    throw!("NotImplemented", msg);
                }
            };
            Ok(keytag)
        }

        let keytag = foo(request)
            .await
            .catch_()
            .map_err(|e| Status::internal(e.to_string()))?;
        Ok(Response::new(keytag))
    }

    async fn ping(&self, _: Request<Void>) -> Result<Response<EchoMessage>, Status> {
        Ok(Response::new(EchoMessage {
            value: crate::version(),
        }))
    }
}
