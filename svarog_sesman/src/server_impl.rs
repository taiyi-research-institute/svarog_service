use std::sync::Arc;

use blake2::digest::{Update, VariableOutput};
use crossbeam_skiplist::SkipMap;
use erreur::*;
use svarog_grpc::{
    mpc_session_manager_server::MpcSessionManager, EchoMessage, Message, SessionConfig, SessionId,
    VecMessage, Void,
};
use tokio::{
    task::JoinHandle,
    time::{sleep, Duration},
};
use tonic::{Request, Response, Status};

pub fn pivot_key() -> [u8; 32] {
    use std::time::{SystemTime, UNIX_EPOCH};
    let now = SystemTime::now();
    let t = now
        .duration_since(UNIX_EPOCH)
        .expect("Time went backwards")
        .as_millis()
        - svarog_sesman::SESSION_EXPIRE_MS;
    let mut pivot = [0u8; 32];
    pivot[..6].copy_from_slice(&t.to_be_bytes()[10..16]);

    pivot
}

pub fn primary_key(sid: &str, topic: &str, src: u64, dst: u64, seq: u64) -> Resultat<[u8; 32]> {
    let mut pk = [0u8; 32];

    // sid
    let sid = hex::decode(sid).catch_()?;
    assert_throw!(sid.len() == 16);
    pk[0..16].copy_from_slice(&sid);

    // message index
    let mut ha = blake2::Blake2bVar::new(16).catch_()?;
    ha.update(format!("{}-{}-{}-{}", topic, src, dst, seq).as_bytes());
    ha.finalize_variable(&mut pk[16..]).catch_()?;

    Ok(pk)
}

#[derive(Clone, Default)]
pub struct Sesman(Arc<SkipMap<[u8; 32], Vec<u8>>>);

impl Sesman {
    pub async fn init() -> Resultat<(Self, JoinHandle<()>)> {
        let sesman = Sesman::default();
        let h = tokio::spawn(sesman.clone().recycle());

        Ok((sesman, h))
    }

    async fn recycle(self) {
        loop {
            let pivot = pivot_key();
            while let Some(entry) = self.0.front() {
                let k = entry.key().clone();
                if k > pivot {
                    // recently added item
                    break;
                } else {
                    // outdated item
                    let _ = entry.remove();
                }
            }
            sleep(Duration::from_secs(60)).await;
        }
    }
}

#[tonic::async_trait]
impl MpcSessionManager for Sesman {
    async fn new_session(
        &self,
        request: Request<SessionConfig>,
    ) -> Result<Response<SessionId>, Status> {
        let mut cfg = request.into_inner();
        if cfg.session_id == "" {
            cfg.session_id = hex::encode(uuid::Uuid::now_v7().as_bytes()).to_lowercase();
        }

        let key = primary_key(&cfg.session_id, "session config", 0, 0, 0)
            .catch_()
            .map_err(|e| Status::internal(e.to_string()))?;
        let val = serde_pickle::to_vec(&cfg, Default::default())
            .catch_()
            .map_err(|e| Status::internal(e.to_string()))?;
        self.0.compare_insert(key, val, |_| true);

        let sid = SessionId {
            value: cfg.session_id.clone(),
        };

        Ok(Response::new(sid))
    }

    async fn get_session_config(
        &self,
        request: Request<SessionId>,
    ) -> Result<Response<SessionConfig>, Status> {
        let sid = request.into_inner().value;
        let key = primary_key(&sid, "session config", 0, 0, 0)
            .catch_()
            .map_err(|e| Status::internal(e.to_string()))?;
        let entry = self
            .0
            .get(&key)
            .ifnone_()
            .map_err(|e| Status::internal(e.to_string()))?;
        let val = entry.value();
        let cfg: SessionConfig = serde_pickle::from_slice(val, Default::default())
            .catch_()
            .map_err(|e| Status::internal(e.to_string()))?;
        Ok(Response::new(cfg))
    }

    async fn inbox(&self, req: Request<VecMessage>) -> Result<Response<Void>, Status> {
        let msgs = req.into_inner().values;
        for msg in msgs.iter() {
            let key = primary_key(&msg.session_id, &msg.topic, msg.src, msg.dst, msg.seq)
                .catch_()
                .map_err(|e| Status::internal(e.to_string()))?;
            let val = msg
                .obj
                .as_ref()
                .ifnone_()
                .map_err(|e| Status::internal(e.to_string()))?
                .clone();
            let _ = self.0.compare_insert(key, val, |_| true);
        }
        Ok(Response::new(Void {}))
    }

    async fn outbox(&self, request: Request<VecMessage>) -> Result<Response<VecMessage>, Status> {
        let idxs = request.into_inner().values;
        let mut resp = Vec::new();
        for idx in idxs.iter() {
            let key = primary_key(&idx.session_id, &idx.topic, idx.src, idx.dst, idx.seq)
                .catch_()
                .map_err(|e| Status::internal(e.to_string()))?;
            let obj = loop {
                let entry = self.0.get(&key);
                match entry {
                    Some(ref_obj) => break ref_obj.value().clone(),
                    None => {
                        sleep(Duration::from_secs(1)).await;
                        continue;
                    }
                };
            };
            resp.push(Message {
                session_id: idx.session_id.clone(),
                topic: idx.topic.clone(),
                src: idx.src,
                dst: idx.dst,
                seq: idx.seq,
                obj: Some(obj),
            })
        }

        Ok(Response::new(VecMessage { values: resp }))
    }

    async fn ping(&self, _: Request<Void>) -> Result<Response<EchoMessage>, Status> {
        Ok(Response::new(EchoMessage {
            value: svarog_sesman::version(),
        }))
    }
}
